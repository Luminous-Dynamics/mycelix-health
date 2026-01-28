//! VCF (Variant Call Format) Parser for HDC Encoding
//!
//! Parses standard VCF files and extracts variants for encoding.
//! Supports both small panel VCFs and whole-exome/genome scale files.
//!
//! # WES/WGS Scale Features
//!
//! - **Streaming Processing**: Iterator-based parsing for memory efficiency
//! - **Chunked Encoding**: Process variants in batches (default 10,000)
//! - **Chromosome-Level Vectors**: Separate vectors per chromosome for better locality
//! - **Parallel Processing**: Multi-threaded encoding via rayon
//! - **gzip Support**: Read .vcf.gz files directly with flate2
//!
//! # Example: Processing a WGS File
//!
//! ```ignore
//! use hdc_core::vcf::{WgsVcfEncoder, WgsEncodingConfig};
//!
//! let config = WgsEncodingConfig::default()
//!     .with_chunk_size(50_000)
//!     .with_parallel(true);
//!
//! let encoder = WgsVcfEncoder::new(Seed::from_string("patient-001"), config);
//! let result = encoder.encode_file("genome.vcf.gz")?;
//!
//! println!("Encoded {} variants across {} chromosomes",
//!     result.total_variants, result.chromosome_vectors.len());
//! ```

use crate::{HdcError, Hypervector, Seed, bundle};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[cfg(feature = "gzip")]
use flate2::read::GzDecoder;

/// A single variant from a VCF file
#[derive(Debug, Clone)]
pub struct Variant {
    /// Chromosome (e.g., "chr1", "1", "X")
    pub chrom: String,
    /// 1-based position
    pub pos: u64,
    /// Variant ID (e.g., "rs1234" or ".")
    pub id: String,
    /// Reference allele
    pub ref_allele: String,
    /// Alternate allele(s)
    pub alt_alleles: Vec<String>,
    /// Quality score (PHRED-scaled)
    pub qual: Option<f64>,
    /// Filter status (PASS or list of filters)
    pub filter: String,
    /// Genotype for the sample (if present)
    pub genotype: Option<Genotype>,
}

/// Genotype information
#[derive(Debug, Clone, PartialEq)]
pub enum Genotype {
    /// Homozygous reference (0/0)
    HomRef,
    /// Heterozygous (0/1 or 1/0)
    Het,
    /// Homozygous alternate (1/1)
    HomAlt,
    /// Multi-allelic or complex (e.g., 1/2)
    Other(String),
    /// Missing genotype (./.)
    Missing,
}

impl Genotype {
    /// Parse genotype from VCF GT field
    pub fn from_gt(gt: &str) -> Self {
        let gt = gt.split(':').next().unwrap_or(gt);
        match gt {
            "0/0" | "0|0" => Genotype::HomRef,
            "0/1" | "1/0" | "0|1" | "1|0" => Genotype::Het,
            "1/1" | "1|1" => Genotype::HomAlt,
            "./." | ".|." => Genotype::Missing,
            other => Genotype::Other(other.to_string()),
        }
    }

    /// Get numeric representation for encoding
    pub fn as_code(&self) -> u8 {
        match self {
            Genotype::HomRef => 0,
            Genotype::Het => 1,
            Genotype::HomAlt => 2,
            Genotype::Missing => 255,
            Genotype::Other(_) => 3,
        }
    }
}

/// VCF file parser
pub struct VcfReader<R: Read> {
    reader: BufReader<R>,
    header_lines: Vec<String>,
    sample_names: Vec<String>,
}

impl<R: Read> VcfReader<R> {
    /// Create a new VCF reader
    pub fn new(reader: R) -> Result<Self, HdcError> {
        let mut buf_reader = BufReader::new(reader);
        let mut header_lines = Vec::new();
        let mut sample_names = Vec::new();
        let mut line = String::new();

        // Read header lines
        loop {
            line.clear();
            let bytes_read = buf_reader.read_line(&mut line)
                .map_err(|e| HdcError::IoError {
                    operation: "read VCF header",
                    message: e.to_string()
                })?;

            if bytes_read == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.starts_with("##") {
                header_lines.push(trimmed.to_string());
            } else if trimmed.starts_with("#CHROM") {
                // Parse column header to get sample names
                let parts: Vec<&str> = trimmed.split('\t').collect();
                if parts.len() > 9 {
                    sample_names = parts[9..].iter().map(|s| s.to_string()).collect();
                }
                break;
            }
        }

        Ok(VcfReader {
            reader: buf_reader,
            header_lines,
            sample_names,
        })
    }

    /// Get sample names
    pub fn sample_names(&self) -> &[String] {
        &self.sample_names
    }

    /// Read all variants
    pub fn read_variants(&mut self) -> Result<Vec<Variant>, HdcError> {
        let mut variants = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)
                .map_err(|e| HdcError::IoError {
                    operation: "read VCF variant line",
                    message: e.to_string()
                })?;

            if bytes_read == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some(variant) = self.parse_variant_line(trimmed) {
                variants.push(variant);
            }
        }

        Ok(variants)
    }

    /// Parse a single variant line
    fn parse_variant_line(&self, line: &str) -> Option<Variant> {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 8 {
            return None;
        }

        let chrom = parts[0].to_string();
        let pos = parts[1].parse().ok()?;
        let id = parts[2].to_string();
        let ref_allele = parts[3].to_string();
        let alt_alleles: Vec<String> = parts[4].split(',').map(|s| s.to_string()).collect();
        let qual = parts[5].parse().ok();
        let filter = parts[6].to_string();

        // Parse genotype if present
        let genotype = if parts.len() > 9 {
            Some(Genotype::from_gt(parts[9]))
        } else {
            None
        };

        Some(Variant {
            chrom,
            pos,
            id,
            ref_allele,
            alt_alleles,
            qual,
            filter,
            genotype,
        })
    }
}

/// VCF-based variant encoder
pub struct VcfEncoder {
    seed: Seed,
}

impl VcfEncoder {
    /// Create a new VCF encoder
    pub fn new(seed: Seed) -> Self {
        VcfEncoder { seed }
    }

    /// Encode a set of variants as a hypervector
    pub fn encode_variants(&self, variants: &[Variant]) -> Result<Hypervector, HdcError> {
        if variants.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let mut variant_vectors: Vec<Hypervector> = Vec::new();

        for variant in variants {
            // Skip variants without genotype or with missing genotype
            let genotype = match &variant.genotype {
                Some(g) if *g != Genotype::Missing => g,
                _ => continue,
            };

            // Create variant identifier
            let variant_key = format!(
                "{}:{}:{}:{}:{}",
                variant.chrom,
                variant.pos,
                variant.ref_allele,
                variant.alt_alleles.join(","),
                genotype.as_code()
            );

            let variant_vec = Hypervector::random(&self.seed, &variant_key);
            variant_vectors.push(variant_vec);
        }

        if variant_vectors.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let refs: Vec<&Hypervector> = variant_vectors.iter().collect();
        Ok(bundle(&refs))
    }

    /// Encode variants with position-based weighting
    /// Variants are weighted by their position to preserve genomic structure
    pub fn encode_variants_positional(&self, variants: &[Variant]) -> Result<Hypervector, HdcError> {
        if variants.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let mut variant_vectors: Vec<Hypervector> = Vec::new();

        for (idx, variant) in variants.iter().enumerate() {
            let genotype = match &variant.genotype {
                Some(g) if *g != Genotype::Missing => g,
                _ => continue,
            };

            // Create base variant vector
            let variant_key = format!(
                "{}:{}:{}:{}",
                variant.chrom,
                variant.ref_allele,
                variant.alt_alleles.join(","),
                genotype.as_code()
            );

            let base_vec = Hypervector::random(&self.seed, &variant_key);

            // Apply positional encoding via permutation
            let pos_vec = base_vec.permute(idx % 1000);
            variant_vectors.push(pos_vec);
        }

        if variant_vectors.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let refs: Vec<&Hypervector> = variant_vectors.iter().collect();
        Ok(bundle(&refs))
    }

    /// Encode only variants matching specific rsIDs
    pub fn encode_panel(&self, variants: &[Variant], rsids: &[&str]) -> Result<Hypervector, HdcError> {
        let rsid_set: std::collections::HashSet<&str> = rsids.iter().copied().collect();

        let filtered: Vec<&Variant> = variants
            .iter()
            .filter(|v| rsid_set.contains(v.id.as_str()))
            .collect();

        if filtered.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let owned: Vec<Variant> = filtered.into_iter().cloned().collect();
        self.encode_variants(&owned)
    }
}

/// Encoded VCF result with metadata
#[derive(Debug, Clone)]
pub struct EncodedVcf {
    /// The hypervector encoding
    pub vector: Hypervector,
    /// Number of variants encoded
    pub variant_count: usize,
    /// Sample name (if available)
    pub sample_name: Option<String>,
    /// Chromosomes represented
    pub chromosomes: Vec<String>,
}

// =============================================================================
// WES/WGS Scale Support
// =============================================================================

/// Configuration for whole-exome/genome scale VCF encoding
#[derive(Debug, Clone)]
pub struct WgsEncodingConfig {
    /// Number of variants to process per chunk (default: 10,000)
    pub chunk_size: usize,
    /// Enable parallel processing with rayon
    pub parallel: bool,
    /// Generate per-chromosome vectors
    pub chromosome_vectors: bool,
    /// Minimum genotype quality (GQ) to include variant
    pub min_gq: Option<u32>,
    /// Only include PASS filter variants
    pub pass_only: bool,
    /// Quality threshold for QUAL field
    pub min_qual: Option<f64>,
    /// Maximum variants to encode (for testing/sampling)
    pub max_variants: Option<usize>,
}

impl Default for WgsEncodingConfig {
    fn default() -> Self {
        WgsEncodingConfig {
            chunk_size: 10_000,
            parallel: true,
            chromosome_vectors: true,
            min_gq: None,
            pass_only: true,
            min_qual: None,
            max_variants: None,
        }
    }
}

impl WgsEncodingConfig {
    /// Set chunk size for batch processing
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Enable/disable parallel processing
    pub fn with_parallel(mut self, enabled: bool) -> Self {
        self.parallel = enabled;
        self
    }

    /// Enable/disable per-chromosome vectors
    pub fn with_chromosome_vectors(mut self, enabled: bool) -> Self {
        self.chromosome_vectors = enabled;
        self
    }

    /// Set minimum genotype quality filter
    pub fn with_min_gq(mut self, gq: u32) -> Self {
        self.min_gq = Some(gq);
        self
    }

    /// Set minimum QUAL filter
    pub fn with_min_qual(mut self, qual: f64) -> Self {
        self.min_qual = Some(qual);
        self
    }

    /// Only include PASS variants
    pub fn with_pass_only(mut self, enabled: bool) -> Self {
        self.pass_only = enabled;
        self
    }

    /// Limit maximum variants (for testing)
    pub fn with_max_variants(mut self, max: usize) -> Self {
        self.max_variants = Some(max);
        self
    }
}

/// Result from WGS/WES encoding
#[derive(Debug, Clone)]
pub struct WgsEncodedResult {
    /// Combined vector across all chromosomes
    pub combined_vector: Hypervector,
    /// Per-chromosome vectors (if chromosome_vectors enabled)
    pub chromosome_vectors: HashMap<String, Hypervector>,
    /// Total variants encoded
    pub total_variants: usize,
    /// Variants per chromosome
    pub variants_per_chromosome: HashMap<String, usize>,
    /// Processing statistics
    pub stats: WgsEncodingStats,
}

/// Statistics from WGS encoding
#[derive(Debug, Clone, Default)]
pub struct WgsEncodingStats {
    /// Total variants read from file
    pub variants_read: usize,
    /// Variants passing filters
    pub variants_passed_filters: usize,
    /// Variants skipped due to missing genotype
    pub skipped_missing_gt: usize,
    /// Variants skipped due to low quality
    pub skipped_low_quality: usize,
    /// Variants skipped due to non-PASS filter
    pub skipped_non_pass: usize,
    /// Number of chunks processed
    pub chunks_processed: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// WGS/WES scale VCF encoder with streaming support
pub struct WgsVcfEncoder {
    seed: Seed,
    config: WgsEncodingConfig,
}

impl WgsVcfEncoder {
    /// Create a new WGS encoder
    pub fn new(seed: Seed, config: WgsEncodingConfig) -> Self {
        WgsVcfEncoder { seed, config }
    }

    /// Encode a VCF file (supports .vcf and .vcf.gz)
    pub fn encode_file<P: AsRef<Path>>(&self, path: P) -> Result<WgsEncodedResult, HdcError> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)
            .map_err(|e| HdcError::IoError {
                operation: "open VCF file",
                message: e.to_string()
            })?;

        // Check for gzip
        let is_gzip = path.extension()
            .map(|ext| ext == "gz")
            .unwrap_or(false);

        if is_gzip {
            #[cfg(feature = "gzip")]
            {
                let decoder = flate2::read::GzDecoder::new(file);
                self.encode_reader(decoder)
            }
            #[cfg(not(feature = "gzip"))]
            {
                Err(HdcError::InvalidConfig {
                    parameter: "gzip",
                    value: path.display().to_string(),
                    reason: "gzip support requires 'gzip' feature. Rebuild with --features gzip".to_string()
                })
            }
        } else {
            self.encode_reader(file)
        }
    }

    /// Encode from any reader with streaming processing
    pub fn encode_reader<R: Read>(&self, reader: R) -> Result<WgsEncodedResult, HdcError> {
        let start_time = std::time::Instant::now();
        let mut stats = WgsEncodingStats::default();

        // Use VariantIterator for streaming
        let variant_iter = VariantIterator::new(reader)?;
        let mut chromosome_variants: HashMap<String, Vec<Hypervector>> = HashMap::new();
        let mut current_chunk: Vec<Variant> = Vec::with_capacity(self.config.chunk_size);
        let mut total_encoded = 0;
        let mut hit_max = false;

        // Process variants in streaming fashion
        for variant_result in variant_iter {
            if hit_max {
                break;
            }

            let variant = match variant_result {
                Ok(v) => v,
                Err(_) => continue, // Skip malformed lines
            };

            stats.variants_read += 1;

            // Apply filters
            if !self.passes_filters(&variant, &mut stats) {
                continue;
            }

            current_chunk.push(variant);

            // Check max variants limit (including current chunk)
            if let Some(max) = self.config.max_variants {
                if total_encoded + current_chunk.len() >= max {
                    hit_max = true;
                }
            }

            // Process chunk when full
            if current_chunk.len() >= self.config.chunk_size {
                self.process_chunk(&current_chunk, &mut chromosome_variants)?;
                total_encoded += current_chunk.len();
                stats.chunks_processed += 1;
                current_chunk.clear();
            }
        }

        // Process remaining variants (potentially trimmed to max)
        if !current_chunk.is_empty() {
            // Trim to max if needed
            if let Some(max) = self.config.max_variants {
                let remaining = max.saturating_sub(total_encoded);
                if remaining < current_chunk.len() {
                    current_chunk.truncate(remaining);
                }
            }

            if !current_chunk.is_empty() {
                self.process_chunk(&current_chunk, &mut chromosome_variants)?;
                total_encoded += current_chunk.len();
                stats.chunks_processed += 1;
            }
        }

        stats.variants_passed_filters = total_encoded;
        stats.processing_time_ms = start_time.elapsed().as_millis() as u64;

        // Combine chromosome vectors
        self.finalize_result(chromosome_variants, stats)
    }

    /// Check if variant passes configured filters
    fn passes_filters(&self, variant: &Variant, stats: &mut WgsEncodingStats) -> bool {
        // Check genotype
        match &variant.genotype {
            Some(g) if *g != Genotype::Missing && *g != Genotype::HomRef => {}
            Some(Genotype::HomRef) => return false, // Skip hom-ref (no variant)
            _ => {
                stats.skipped_missing_gt += 1;
                return false;
            }
        }

        // Check PASS filter
        if self.config.pass_only && variant.filter != "PASS" && variant.filter != "." {
            stats.skipped_non_pass += 1;
            return false;
        }

        // Check QUAL
        if let Some(min_qual) = self.config.min_qual {
            if variant.qual.map(|q| q < min_qual).unwrap_or(true) {
                stats.skipped_low_quality += 1;
                return false;
            }
        }

        true
    }

    /// Process a chunk of variants
    fn process_chunk(
        &self,
        variants: &[Variant],
        chromosome_variants: &mut HashMap<String, Vec<Hypervector>>,
    ) -> Result<(), HdcError> {
        // Group by chromosome
        let mut by_chrom: HashMap<String, Vec<&Variant>> = HashMap::new();
        for variant in variants {
            by_chrom.entry(variant.chrom.clone())
                .or_default()
                .push(variant);
        }

        // Encode each chromosome group
        #[cfg(feature = "parallel")]
        {
            if self.config.parallel {
                use rayon::prelude::*;

                let results: Vec<_> = by_chrom.par_iter()
                    .map(|(chrom, vars)| {
                        let vectors = self.encode_variant_batch(vars);
                        (chrom.clone(), vectors)
                    })
                    .collect();

                for (chrom, vectors) in results {
                    chromosome_variants.entry(chrom)
                        .or_default()
                        .extend(vectors);
                }
            } else {
                // Sequential when parallel disabled in config
                for (chrom, vars) in by_chrom {
                    let vectors = self.encode_variant_batch(&vars);
                    chromosome_variants.entry(chrom)
                        .or_default()
                        .extend(vectors);
                }
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            for (chrom, vars) in by_chrom {
                let vectors = self.encode_variant_batch(&vars);
                chromosome_variants.entry(chrom)
                    .or_default()
                    .extend(vectors);
            }
        }

        Ok(())
    }

    /// Encode a batch of variants to hypervectors
    fn encode_variant_batch(&self, variants: &[&Variant]) -> Vec<Hypervector> {
        variants.iter()
            .filter_map(|v| {
                let genotype = v.genotype.as_ref()?;
                let variant_key = format!(
                    "{}:{}:{}:{}:{}",
                    v.chrom, v.pos, v.ref_allele,
                    v.alt_alleles.join(","),
                    genotype.as_code()
                );
                Some(Hypervector::random(&self.seed, &variant_key))
            })
            .collect()
    }

    /// Finalize and combine chromosome vectors
    fn finalize_result(
        &self,
        chromosome_variants: HashMap<String, Vec<Hypervector>>,
        stats: WgsEncodingStats,
    ) -> Result<WgsEncodedResult, HdcError> {
        let mut chromosome_vectors: HashMap<String, Hypervector> = HashMap::new();
        let mut variants_per_chromosome: HashMap<String, usize> = HashMap::new();
        let mut all_vectors: Vec<Hypervector> = Vec::new();
        let mut total_variants = 0;

        // Bundle each chromosome's variants
        for (chrom, vectors) in chromosome_variants {
            if vectors.is_empty() {
                continue;
            }

            let count = vectors.len();
            variants_per_chromosome.insert(chrom.clone(), count);
            total_variants += count;

            // Bundle chromosome vectors
            let refs: Vec<&Hypervector> = vectors.iter().collect();
            let chrom_vector = bundle(&refs);

            if self.config.chromosome_vectors {
                chromosome_vectors.insert(chrom, chrom_vector.clone());
            }
            all_vectors.push(chrom_vector);
        }

        // Create combined vector
        let combined_vector = if all_vectors.is_empty() {
            return Err(HdcError::EmptyInput);
        } else if all_vectors.len() == 1 {
            all_vectors.remove(0)
        } else {
            let refs: Vec<&Hypervector> = all_vectors.iter().collect();
            bundle(&refs)
        };

        Ok(WgsEncodedResult {
            combined_vector,
            chromosome_vectors,
            total_variants,
            variants_per_chromosome,
            stats,
        })
    }
}

/// Iterator adapter for streaming variant processing
pub struct VariantIterator<R: Read> {
    reader: BufReader<R>,
    line_buffer: String,
    sample_names: Vec<String>,
}

impl<R: Read> VariantIterator<R> {
    /// Create a new variant iterator
    pub fn new(reader: R) -> Result<Self, HdcError> {
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();
        let mut sample_names = Vec::new();

        // Skip header, capture sample names
        loop {
            line.clear();
            let bytes_read = buf_reader.read_line(&mut line)
                .map_err(|e| HdcError::IoError {
                    operation: "read VCF header in iterator",
                    message: e.to_string()
                })?;

            if bytes_read == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.starts_with("#CHROM") {
                let parts: Vec<&str> = trimmed.split('\t').collect();
                if parts.len() > 9 {
                    sample_names = parts[9..].iter().map(|s| s.to_string()).collect();
                }
                break;
            }
        }

        Ok(VariantIterator {
            reader: buf_reader,
            line_buffer: String::new(),
            sample_names,
        })
    }

    /// Get sample names
    pub fn sample_names(&self) -> &[String] {
        &self.sample_names
    }
}

impl<R: Read> Iterator for VariantIterator<R> {
    type Item = Result<Variant, HdcError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.line_buffer.clear();
            match self.reader.read_line(&mut self.line_buffer) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    let trimmed = self.line_buffer.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    return Some(parse_variant_line_static(trimmed));
                }
                Err(e) => return Some(Err(HdcError::IoError {
                    operation: "read VCF variant line",
                    message: e.to_string()
                })),
            }
        }
    }
}

/// Parse a variant line (standalone function for iterator)
fn parse_variant_line_static(line: &str) -> Result<Variant, HdcError> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 8 {
        return Err(HdcError::VcfFormatError {
            line_number: None,
            message: format!("too few fields (expected 8+, got {})", parts.len())
        });
    }

    let pos = parts[1].parse()
        .map_err(|_| HdcError::VcfFormatError {
            line_number: None,
            message: format!("invalid position value: '{}'", parts[1])
        })?;

    let genotype = if parts.len() > 9 {
        Some(Genotype::from_gt(parts[9]))
    } else {
        None
    };

    Ok(Variant {
        chrom: parts[0].to_string(),
        pos,
        id: parts[2].to_string(),
        ref_allele: parts[3].to_string(),
        alt_alleles: parts[4].split(',').map(|s| s.to_string()).collect(),
        qual: parts[5].parse().ok(),
        filter: parts[6].to_string(),
        genotype,
    })
}

/// Region specification for targeted encoding
#[derive(Debug, Clone)]
pub struct GenomicRegion {
    /// Chromosome
    pub chrom: String,
    /// Start position (1-based, inclusive)
    pub start: u64,
    /// End position (1-based, inclusive)
    pub end: u64,
}

impl GenomicRegion {
    /// Create a new genomic region
    pub fn new(chrom: &str, start: u64, end: u64) -> Self {
        GenomicRegion {
            chrom: chrom.to_string(),
            start,
            end,
        }
    }

    /// Check if a variant falls within this region
    pub fn contains(&self, variant: &Variant) -> bool {
        variant.chrom == self.chrom
            && variant.pos >= self.start
            && variant.pos <= self.end
    }

    /// Parse from string "chr1:1000-2000"
    pub fn from_string(s: &str) -> Result<Self, HdcError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(HdcError::InvalidRegion {
                input: s.to_string(),
                reason: "expected format 'chr:start-end'".to_string()
            });
        }

        let chrom = parts[0].to_string();
        let range_parts: Vec<&str> = parts[1].split('-').collect();
        if range_parts.len() != 2 {
            return Err(HdcError::InvalidRegion {
                input: s.to_string(),
                reason: "expected format 'start-end' for range".to_string()
            });
        }

        let start = range_parts[0].parse()
            .map_err(|_| HdcError::InvalidRegion {
                input: s.to_string(),
                reason: format!("invalid start position: '{}'", range_parts[0])
            })?;
        let end = range_parts[1].parse()
            .map_err(|_| HdcError::InvalidRegion {
                input: s.to_string(),
                reason: format!("invalid end position: '{}'", range_parts[1])
            })?;

        Ok(GenomicRegion { chrom, start, end })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const TEST_VCF: &str = r#"##fileformat=VCFv4.2
##INFO=<ID=DP,Number=1,Type=Integer,Description="Total Depth">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	SAMPLE1
chr1	100	rs123	A	G	30	PASS	DP=10	GT	0/1
chr1	200	rs456	C	T	40	PASS	DP=20	GT	1/1
chr2	300	rs789	G	A	50	PASS	DP=15	GT	0/0
"#;

    #[test]
    fn test_vcf_parsing() {
        let cursor = Cursor::new(TEST_VCF);
        let mut reader = VcfReader::new(cursor).unwrap();
        let variants = reader.read_variants().unwrap();

        assert_eq!(variants.len(), 3);
        assert_eq!(variants[0].id, "rs123");
        assert_eq!(variants[0].genotype, Some(Genotype::Het));
        assert_eq!(variants[1].genotype, Some(Genotype::HomAlt));
        assert_eq!(variants[2].genotype, Some(Genotype::HomRef));
    }

    #[test]
    fn test_vcf_encoding() {
        let cursor = Cursor::new(TEST_VCF);
        let mut reader = VcfReader::new(cursor).unwrap();
        let variants = reader.read_variants().unwrap();

        let seed = Seed::from_string("test-vcf");
        let encoder = VcfEncoder::new(seed);
        let encoded = encoder.encode_variants(&variants).unwrap();

        assert_eq!(encoded.as_bytes().len(), crate::HYPERVECTOR_BYTES);
    }

    // Large VCF for WGS testing (simulates multi-chromosome data)
    const LARGE_TEST_VCF: &str = r#"##fileformat=VCFv4.2
##INFO=<ID=DP,Number=1,Type=Integer,Description="Total Depth">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	SAMPLE1
chr1	100	rs001	A	G	30	PASS	DP=10	GT	0/1
chr1	200	rs002	C	T	40	PASS	DP=20	GT	1/1
chr1	300	rs003	G	A	50	PASS	DP=15	GT	0/1
chr2	100	rs004	T	C	35	PASS	DP=12	GT	0/1
chr2	200	rs005	A	G	45	PASS	DP=18	GT	1/1
chr2	300	rs006	C	T	25	LowQual	DP=8	GT	0/1
chr3	100	rs007	G	A	55	PASS	DP=22	GT	0/1
chr3	200	rs008	T	C	.	PASS	DP=5	GT	0/0
chrX	100	rs009	A	T	60	PASS	DP=30	GT	1/1
chrX	200	rs010	C	G	70	PASS	DP=35	GT	0/1
"#;

    #[test]
    fn test_wgs_encoder_basic() {
        let cursor = Cursor::new(LARGE_TEST_VCF);
        let seed = Seed::from_string("wgs-test");
        let config = WgsEncodingConfig::default()
            .with_chunk_size(5)
            .with_parallel(false);

        let encoder = WgsVcfEncoder::new(seed, config);
        let result = encoder.encode_reader(cursor).unwrap();

        // Should encode Het and HomAlt variants from PASS filter
        // Skips: HomRef (rs008), LowQual (rs006)
        assert!(result.total_variants >= 6);
        assert!(!result.chromosome_vectors.is_empty());
        assert!(result.chromosome_vectors.contains_key("chr1"));
        assert!(result.chromosome_vectors.contains_key("chr2"));
    }

    #[test]
    fn test_wgs_encoder_filtering() {
        let cursor = Cursor::new(LARGE_TEST_VCF);
        let seed = Seed::from_string("filter-test");
        let config = WgsEncodingConfig::default()
            .with_pass_only(true)
            .with_parallel(false);

        let encoder = WgsVcfEncoder::new(seed, config);
        let result = encoder.encode_reader(cursor).unwrap();

        // rs006 should be filtered (LowQual)
        assert!(result.stats.skipped_non_pass >= 1);
    }

    #[test]
    fn test_wgs_encoder_max_variants() {
        let cursor = Cursor::new(LARGE_TEST_VCF);
        let seed = Seed::from_string("max-test");
        let config = WgsEncodingConfig::default()
            .with_max_variants(3)
            .with_parallel(false);

        let encoder = WgsVcfEncoder::new(seed, config);
        let result = encoder.encode_reader(cursor).unwrap();

        // Should encode exactly 3 variants (truncated to max)
        assert_eq!(result.total_variants, 3, "Expected exactly 3 variants, got {}", result.total_variants);
    }

    #[test]
    fn test_wgs_encoder_chromosome_vectors() {
        let cursor = Cursor::new(LARGE_TEST_VCF);
        let seed = Seed::from_string("chrom-test");
        let config = WgsEncodingConfig::default()
            .with_chromosome_vectors(true)
            .with_parallel(false);

        let encoder = WgsVcfEncoder::new(seed, config);
        let result = encoder.encode_reader(cursor).unwrap();

        // Check per-chromosome vectors exist
        assert!(result.chromosome_vectors.len() >= 3);
        for (chrom, count) in &result.variants_per_chromosome {
            if *count > 0 {
                assert!(result.chromosome_vectors.contains_key(chrom));
            }
        }
    }

    #[test]
    fn test_variant_iterator() {
        let cursor = Cursor::new(TEST_VCF);
        let iter = VariantIterator::new(cursor).unwrap();

        let variants: Vec<_> = iter.filter_map(|r| r.ok()).collect();
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn test_genomic_region() {
        let region = GenomicRegion::from_string("chr1:100-500").unwrap();
        assert_eq!(region.chrom, "chr1");
        assert_eq!(region.start, 100);
        assert_eq!(region.end, 500);

        let variant = Variant {
            chrom: "chr1".to_string(),
            pos: 200,
            id: "rs123".to_string(),
            ref_allele: "A".to_string(),
            alt_alleles: vec!["G".to_string()],
            qual: Some(30.0),
            filter: "PASS".to_string(),
            genotype: Some(Genotype::Het),
        };

        assert!(region.contains(&variant));

        let outside = Variant {
            chrom: "chr1".to_string(),
            pos: 600,
            ..variant.clone()
        };
        assert!(!region.contains(&outside));

        let wrong_chrom = Variant {
            chrom: "chr2".to_string(),
            pos: 200,
            ..variant
        };
        assert!(!region.contains(&wrong_chrom));
    }

    #[test]
    fn test_wgs_deterministic() {
        // Same input should produce same output
        let seed = Seed::from_string("deterministic-test");
        let config = WgsEncodingConfig::default().with_parallel(false);

        let cursor1 = Cursor::new(LARGE_TEST_VCF);
        let encoder1 = WgsVcfEncoder::new(seed.clone(), config.clone());
        let result1 = encoder1.encode_reader(cursor1).unwrap();

        let cursor2 = Cursor::new(LARGE_TEST_VCF);
        let encoder2 = WgsVcfEncoder::new(seed, config);
        let result2 = encoder2.encode_reader(cursor2).unwrap();

        assert_eq!(result1.combined_vector, result2.combined_vector);
        assert_eq!(result1.total_variants, result2.total_variants);
    }
}
