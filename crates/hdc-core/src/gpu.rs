// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! GPU Acceleration for HDC Similarity Computation
//!
//! Uses wgpu for cross-platform GPU acceleration (Vulkan/Metal/DirectX/WebGPU).
//! Enables massive parallel similarity computation for large-scale matching.
//!
//! # Performance
//!
//! GPU acceleration is beneficial for:
//! - Batch similarity computations (1000s of comparisons)
//! - Large database searches
//! - Real-time matching applications
//!
//! # Example
//!
//! ```ignore
//! use hdc_core::gpu::GpuSimilarityEngine;
//!
//! let engine = GpuSimilarityEngine::new().await?;
//!
//! // Batch compute similarities
//! let similarities = engine.batch_similarity(&queries, &database).await?;
//! ```

use crate::{Hypervector, HYPERVECTOR_BYTES};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Errors that can occur during GPU operations
#[derive(Debug, Clone)]
pub enum GpuError {
    /// No suitable GPU adapter found
    NoAdapter,
    /// Failed to get GPU device
    DeviceError(String),
    /// Shader compilation error
    ShaderError(String),
    /// Buffer size mismatch
    BufferError(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::NoAdapter => write!(f, "No suitable GPU adapter found"),
            GpuError::DeviceError(e) => write!(f, "GPU device error: {}", e),
            GpuError::ShaderError(e) => write!(f, "Shader error: {}", e),
            GpuError::BufferError(e) => write!(f, "Buffer error: {}", e),
        }
    }
}

impl std::error::Error for GpuError {}

/// Pack each fixed-width hypervector into the u32-addressed layout used by WGSL.
///
/// Storage buffers are interpreted as array<u32>; each vector therefore needs its
/// own four-byte-rounded stride. Padding bytes are zero and are excluded from the
/// semantic bit count passed to the shader.
fn pack_hypervectors_u32_aligned(vectors: &[Hypervector]) -> Result<Vec<u8>, GpuError> {
    let stride = HYPERVECTOR_BYTES
        .checked_add(3)
        .map(|bytes| bytes & !3usize)
        .ok_or_else(|| GpuError::BufferError("hypervector stride overflow".into()))?;
    let total_size = stride
        .checked_mul(vectors.len())
        .ok_or_else(|| GpuError::BufferError("packed hypervector buffer size overflow".into()))?;
    let mut packed = vec![0_u8; total_size];

    for (index, vector) in vectors.iter().enumerate() {
        let bytes = vector.as_bytes();
        if bytes.len() != HYPERVECTOR_BYTES {
            return Err(GpuError::BufferError(format!(
                "hypervector {index} has {} bytes; expected {HYPERVECTOR_BYTES}",
                bytes.len()
            )));
        }
        let offset = index * stride;
        packed[offset..offset + HYPERVECTOR_BYTES].copy_from_slice(bytes);
    }

    Ok(packed)
}

fn checked_comparison_count(query_count: u32, database_count: u32) -> Result<(u32, usize), GpuError> {
    let count = query_count.checked_mul(database_count).ok_or_else(|| {
        GpuError::BufferError("query_count * database_count exceeds the shader's u32 index space".into())
    })?;
    let count_usize = usize::try_from(count)
        .map_err(|_| GpuError::BufferError("comparison count does not fit host usize".into()))?;
    Ok((count, count_usize))
}

/// GPU-accelerated similarity computation engine
pub struct GpuSimilarityEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    adapter_info: wgpu::AdapterInfo,
}

impl GpuSimilarityEngine {
    /// Create a new GPU similarity engine
    pub async fn new() -> Result<Self, GpuError> {
        Self::new_with_backends(wgpu::Backends::all()).await
    }

    async fn new_with_backends(backends: wgpu::Backends) -> Result<Self, GpuError> {
        // The qualification test can pin this to Vulkan; normal callers retain
        // the existing cross-platform backend selection behavior.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuError::NoAdapter)?;
        let adapter_info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("HDC Similarity Engine"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| GpuError::DeviceError(e.to_string()))?;

        // Create compute shader for Hamming similarity
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HDC Similarity Shader"),
            source: wgpu::ShaderSource::Wgsl(SIMILARITY_SHADER.into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HDC Bind Group Layout"),
            entries: &[
                // Query vectors buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Database vectors buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output similarities buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Params buffer (query_count, db_count)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("HDC Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("HDC Similarity Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("compute_similarity"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(GpuSimilarityEngine {
            device,
            queue,
            pipeline,
            bind_group_layout,
            adapter_info,
        })
    }

    /// Compute similarities between all queries and all database vectors
    ///
    /// Returns a flattened array of similarities: output[i * db_count + j] = similarity(query[i], db[j])
    pub async fn batch_similarity(
        &self,
        queries: &[Hypervector],
        database: &[Hypervector],
    ) -> Result<Vec<f32>, GpuError> {
        if queries.is_empty() || database.is_empty() {
            return Ok(Vec::new());
        }

        let query_count = u32::try_from(queries.len()).map_err(|_| {
            GpuError::BufferError("query count exceeds the shader's u32 index space".into())
        })?;
        let db_count = u32::try_from(database.len()).map_err(|_| {
            GpuError::BufferError("database count exceeds the shader's u32 index space".into())
        })?;
        let (output_count_u32, output_count) = checked_comparison_count(query_count, db_count)?;

        // The shader indexes each Hypervector as array<u32>. With the current
        // 1,250-byte HDC representation, flat concatenation would shift every
        // vector after the first by two bytes and overrun the final vector.
        let query_data = pack_hypervectors_u32_aligned(queries)?;
        let db_data = pack_hypervectors_u32_aligned(database)?;

        let limits = self.device.limits();
        let max_storage_binding_size = limits.max_storage_buffer_binding_size as usize;
        for (label, data) in [("query", &query_data), ("database", &db_data)] {
            if data.len() > max_storage_binding_size {
                return Err(GpuError::BufferError(format!(
                    "{label} buffer size {} exceeds max_storage_buffer_binding_size {}",
                    data.len(), limits.max_storage_buffer_binding_size
                )));
            }
        }
        let output_size = u64::try_from(output_count)
            .ok()
            .and_then(|count| count.checked_mul(std::mem::size_of::<f32>() as u64))
            .ok_or_else(|| GpuError::BufferError("similarity output byte size overflow".into()))?;
        if output_size > u64::from(limits.max_storage_buffer_binding_size) {
            return Err(GpuError::BufferError(format!(
                "output buffer size {output_size} exceeds max_storage_buffer_binding_size {}",
                limits.max_storage_buffer_binding_size
            )));
        }
        let workgroup_size = 64_u32;
        let num_workgroups = output_count_u32.div_ceil(workgroup_size);
        if num_workgroups > limits.max_compute_workgroups_per_dimension {
            return Err(GpuError::BufferError(format!(
                "dispatch requires {num_workgroups} workgroups, device limit is {}",
                limits.max_compute_workgroups_per_dimension
            )));
        }

        // Create buffers
        let query_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Query Buffer"),
                contents: &query_data,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let db_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Database Buffer"),
                contents: &db_data,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Params: [query_count, db_count, vector_bytes, _padding]
        let params: [u32; 4] = [query_count, db_count, HYPERVECTOR_BYTES as u32, 0];
        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Params Buffer"),
                contents: bytemuck::cast_slice(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HDC Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: query_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: db_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Submit compute pass
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("HDC Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("HDC Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups: one thread per (query, db) pair
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Copy output to staging buffer
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_size);

        self.queue.submit(Some(encoder.finish()));

        // Read results
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .unwrap()
            .map_err(|e| GpuError::BufferError(e.to_string()))?;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }

    /// Find top-K most similar vectors from database for each query
    pub async fn top_k_similarity(
        &self,
        queries: &[Hypervector],
        database: &[Hypervector],
        k: usize,
    ) -> Result<Vec<Vec<(usize, f32)>>, GpuError> {
        let all_similarities = self.batch_similarity(queries, database).await?;
        let db_count = database.len();

        let mut results = Vec::with_capacity(queries.len());

        for q_idx in 0..queries.len() {
            // Get similarities for this query
            let start = q_idx * db_count;
            let end = start + db_count;
            let sims: Vec<(usize, f32)> = all_similarities[start..end]
                .iter()
                .enumerate()
                .map(|(idx, &sim)| (idx, sim))
                .collect();

            // Sort by similarity (descending) and take top-k
            let mut sorted = sims;
            sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            sorted.truncate(k);

            results.push(sorted);
        }

        Ok(results)
    }

    /// Get device info for debugging
    pub fn device_info(&self) -> String {
        format!(
            "GPU Adapter: {:?}; Device limits: {:?}",
            self.adapter_info,
            self.device.limits()
        )
    }
}

/// Synchronous wrapper using pollster
pub mod sync {
    use super::*;

    /// Create a GPU similarity engine synchronously
    pub fn create_engine() -> Result<GpuSimilarityEngine, GpuError> {
        pollster::block_on(GpuSimilarityEngine::new())
    }

    /// Compute batch similarities synchronously
    pub fn batch_similarity(
        engine: &GpuSimilarityEngine,
        queries: &[Hypervector],
        database: &[Hypervector],
    ) -> Result<Vec<f32>, GpuError> {
        pollster::block_on(engine.batch_similarity(queries, database))
    }

    /// Find top-K similar vectors synchronously
    pub fn top_k_similarity(
        engine: &GpuSimilarityEngine,
        queries: &[Hypervector],
        database: &[Hypervector],
        k: usize,
    ) -> Result<Vec<Vec<(usize, f32)>>, GpuError> {
        pollster::block_on(engine.top_k_similarity(queries, database, k))
    }
}

/// WGSL Compute shader for Hamming similarity
const SIMILARITY_SHADER: &str = r#"
// Params: query_count, db_count, vector_bytes, padding
struct Params {
    query_count: u32,
    db_count: u32,
    vector_bytes: u32,
    padding: u32,
}

@group(0) @binding(0) var<storage, read> queries: array<u32>;
@group(0) @binding(1) var<storage, read> database: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

// Popcount for u32
fn popcount(x: u32) -> u32 {
    var v = x;
    v = v - ((v >> 1u) & 0x55555555u);
    v = (v & 0x33333333u) + ((v >> 2u) & 0x33333333u);
    v = (v + (v >> 4u)) & 0x0f0f0f0fu;
    v = v + (v >> 8u);
    v = v + (v >> 16u);
    return v & 0x3fu;
}

@compute @workgroup_size(64)
fn compute_similarity(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total = params.query_count * params.db_count;

    if (idx >= total) {
        return;
    }

    let query_idx = idx / params.db_count;
    let db_idx = idx % params.db_count;

    // Vector size in u32 words (1250 bytes = 313 u32s, rounding up)
    let words_per_vector = (params.vector_bytes + 3u) / 4u;

    let query_start = query_idx * words_per_vector;
    let db_start = db_idx * words_per_vector;

    // Compute Hamming distance via XOR + popcount
    var diff_bits: u32 = 0u;
    for (var i: u32 = 0u; i < words_per_vector; i = i + 1u) {
        let q = queries[query_start + i];
        let d = database[db_start + i];
        diff_bits = diff_bits + popcount(q ^ d);
    }

    // Convert to similarity: 1 - (diff_bits / total_bits)
    let total_bits = params.vector_bytes * 8u;
    let similarity = 1.0 - (f32(diff_bits) / f32(total_bits));

    output[idx] = similarity;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Seed;

    #[test]
    fn shader_upload_pads_each_hypervector_to_a_u32_stride() {
        let seed = Seed::from_string("u32-stride-regression");
        let vectors: Vec<Hypervector> = (0..3)
            .map(|i| Hypervector::random(&seed, &format!("vector-{i}")))
            .collect();
        let packed = pack_hypervectors_u32_aligned(&vectors).unwrap();
        let stride = HYPERVECTOR_BYTES.checked_add(3).unwrap() & !3usize;

        assert_eq!(packed.len(), stride * vectors.len());
        for (index, vector) in vectors.iter().enumerate() {
            let start = index * stride;
            assert_eq!(
                &packed[start..start + HYPERVECTOR_BYTES],
                vector.as_bytes(),
                "vector {index} must start at its own aligned stride"
            );
            assert!(
                packed[start + HYPERVECTOR_BYTES..start + stride]
                    .iter()
                    .all(|byte| *byte == 0),
                "vector {index} padding must be zero-filled"
            );
        }
    }

    #[test]
    fn comparison_count_rejects_shader_u32_overflow() {
        assert_eq!(checked_comparison_count(3, 7).unwrap(), (21, 21));
        assert!(checked_comparison_count(u32::MAX, 2).is_err());
        assert!(checked_comparison_count(u32::MAX, 1).is_ok());
    }

    fn reference_hamming_similarity(lhs: &Hypervector, rhs: &Hypervector) -> f32 {
        let differing_bits: u32 = lhs
            .as_bytes()
            .iter()
            .zip(rhs.as_bytes())
            .map(|(&left, &right)| (left ^ right).count_ones())
            .sum();
        1.0 - differing_bits as f32 / (HYPERVECTOR_BYTES * 8) as f32
    }

    #[test]
    fn test_gpu_engine_creation() {
        // This test requires a GPU, so it may fail in CI
        let result = sync::create_engine();
        match result {
            Ok(engine) => {
                println!("GPU engine created: {}", engine.device_info());
            }
            Err(GpuError::NoAdapter) => {
                println!("No GPU adapter available (expected in headless/CI)");
            }
            Err(e) => {
                println!("GPU error: {}", e);
            }
        }
    }

    #[test]
    fn test_batch_similarity() {
        let require_adapter = std::env::var_os("MYCELIX_REQUIRE_WGPU_ADAPTER").is_some();
        let require_vulkan = std::env::var_os("MYCELIX_REQUIRE_WGPU_VULKAN").is_some();
        let engine_result = if require_vulkan {
            pollster::block_on(GpuSimilarityEngine::new_with_backends(wgpu::Backends::VULKAN))
        } else {
            sync::create_engine()
        };
        let engine = match engine_result {
            Ok(engine) => engine,
            Err(GpuError::NoAdapter) if !require_adapter => {
                println!("GPU adapter unavailable; skip is allowed in generic environments");
                return;
            }
            Err(error) => panic!("GPU engine creation failed: {error}"),
        };
        if require_vulkan {
            assert_eq!(
                engine.adapter_info.backend,
                wgpu::Backend::Vulkan,
                "Vulkan qualification must select the Vulkan backend"
            );
        }
        println!("selected_adapter={:?}", engine.adapter_info);

        let seed = Seed::from_string("test");
        let queries: Vec<Hypervector> = (0..10)
            .map(|i| Hypervector::random(&seed, &format!("query_{}", i)))
            .collect();

        let database: Vec<Hypervector> = (0..100)
            .map(|i| Hypervector::random(&seed, &format!("db_{}", i)))
            .collect();

        let similarities = sync::batch_similarity(&engine, &queries, &database).unwrap();

        assert_eq!(similarities.len(), 10 * 100);

        // Compare every GPU result with a CPU oracle. This checks vector word
        // boundaries, bit-count normalization, pair indexing, and device output.
        for (query_index, query) in queries.iter().enumerate() {
            for (db_index, candidate) in database.iter().enumerate() {
                let expected = reference_hamming_similarity(query, candidate);
                let observed = similarities[query_index * database.len() + db_index];
                assert!(
                    (observed - expected).abs() <= 1.0e-6,
                    "GPU/CPU mismatch at query {query_index}, database {db_index}: observed={observed}, expected={expected}"
                );
            }
        }
    }
}
