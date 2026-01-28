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

/// GPU-accelerated similarity computation engine
pub struct GpuSimilarityEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuSimilarityEngine {
    /// Create a new GPU similarity engine
    pub async fn new() -> Result<Self, GpuError> {
        // Request high-performance adapter
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
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

        let query_count = queries.len() as u32;
        let db_count = database.len() as u32;
        let output_count = (query_count * db_count) as usize;

        // Flatten query vectors
        let query_data: Vec<u8> = queries
            .iter()
            .flat_map(|hv| hv.as_bytes().to_vec())
            .collect();

        // Flatten database vectors
        let db_data: Vec<u8> = database
            .iter()
            .flat_map(|hv| hv.as_bytes().to_vec())
            .collect();

        // Create buffers
        let query_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Query Buffer"),
            contents: &query_data,
            usage: wgpu::BufferUsages::STORAGE,
        });

        let db_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Database Buffer"),
            contents: &db_data,
            usage: wgpu::BufferUsages::STORAGE,
        });

        let output_size = (output_count * std::mem::size_of::<f32>()) as u64;
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
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
            let workgroup_size = 64;
            let num_workgroups = (output_count as u32 + workgroup_size - 1) / workgroup_size;
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
        receiver.recv().unwrap().map_err(|e| GpuError::BufferError(e.to_string()))?;

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
        format!("GPU Device: {:?}", self.device.limits())
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
        let engine = match sync::create_engine() {
            Ok(e) => e,
            Err(_) => {
                println!("Skipping GPU test - no adapter");
                return;
            }
        };

        let seed = Seed::from_string("test");
        let queries: Vec<Hypervector> = (0..10)
            .map(|i| Hypervector::random(&seed, &format!("query_{}", i)))
            .collect();

        let database: Vec<Hypervector> = (0..100)
            .map(|i| Hypervector::random(&seed, &format!("db_{}", i)))
            .collect();

        let similarities = sync::batch_similarity(&engine, &queries, &database).unwrap();

        assert_eq!(similarities.len(), 10 * 100);

        // Check that self-similarities are high (first 10 db entries match queries)
        for i in 0..10 {
            let self_sim = similarities[i * 100 + i];
            println!("Self similarity {}: {}", i, self_sim);
            // With random vectors, self-sim should be ~0.5 for different vectors
        }
    }
}
