//! VCF (Variant Call Format) Parser for HDC Encoding
//!
//! Parses standard VCF files and extracts variants for encoding.

use crate::{HdcError, Hypervector, Seed, bundle};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

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
                .map_err(|e| HdcError::Other(format!("IO error: {}", e)))?;

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
                .map_err(|e| HdcError::Other(format!("IO error: {}", e)))?;

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
}
