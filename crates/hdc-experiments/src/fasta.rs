//! FASTA file parsing for real biological data

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// A parsed FASTA sequence
#[derive(Clone, Debug)]
pub struct FastaSequence {
    pub id: String,
    pub species: String,
    pub marker: String,
    pub accession: Option<String>,
    pub sequence: String,
}

/// Parse a BOLD-formatted FASTA file
/// Format: >ID|Species|Marker|Accession (optional)
pub fn parse_bold_fasta(path: &Path) -> Result<Vec<FastaSequence>, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut sequences = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_seq = String::new();

    for line in reader.lines() {
        let line = line?;
        if line.starts_with('>') {
            // Save previous sequence if exists
            if let Some(header) = current_header.take() {
                if let Some(seq) = parse_bold_header(&header, &current_seq) {
                    sequences.push(seq);
                }
            }
            current_header = Some(line[1..].to_string());
            current_seq.clear();
        } else {
            // Append sequence data (remove gaps and whitespace)
            current_seq.push_str(&line.replace('-', "").replace(' ', ""));
        }
    }

    // Don't forget the last sequence
    if let Some(header) = current_header {
        if let Some(seq) = parse_bold_header(&header, &current_seq) {
            sequences.push(seq);
        }
    }

    Ok(sequences)
}

fn parse_bold_header(header: &str, sequence: &str) -> Option<FastaSequence> {
    if sequence.is_empty() || sequence.len() < 100 {
        return None; // Skip short/empty sequences
    }

    let parts: Vec<&str> = header.split('|').collect();

    if parts.len() >= 3 {
        Some(FastaSequence {
            id: parts[0].to_string(),
            species: parts[1].to_string(),
            marker: parts[2].to_string(),
            accession: parts.get(3).map(|s| s.to_string()),
            sequence: sequence.to_uppercase(),
        })
    } else {
        // Fallback for non-standard format
        Some(FastaSequence {
            id: parts.get(0).unwrap_or(&"unknown").to_string(),
            species: parts.get(1).unwrap_or(&"Unknown species").to_string(),
            marker: "COI-5P".to_string(),
            accession: None,
            sequence: sequence.to_uppercase(),
        })
    }
}

/// Parse an IMGT/HLA-formatted FASTA file
/// Format: >HLA:ID Allele*Field:Field:... Length bp
pub fn parse_imgt_hla_fasta(path: &Path) -> Result<Vec<HlaAlleleSequence>, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut alleles = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_seq = String::new();

    for line in reader.lines() {
        let line = line?;
        if line.starts_with('>') {
            if let Some(header) = current_header.take() {
                if let Some(allele) = parse_hla_header(&header, &current_seq) {
                    alleles.push(allele);
                }
            }
            current_header = Some(line[1..].to_string());
            current_seq.clear();
        } else {
            current_seq.push_str(&line.trim());
        }
    }

    if let Some(header) = current_header {
        if let Some(allele) = parse_hla_header(&header, &current_seq) {
            alleles.push(allele);
        }
    }

    Ok(alleles)
}

/// Parsed HLA allele sequence
#[derive(Clone, Debug)]
pub struct HlaAlleleSequence {
    pub hla_id: String,
    pub allele_name: String,
    pub locus: String,
    pub sequence: String,
}

fn parse_hla_header(header: &str, sequence: &str) -> Option<HlaAlleleSequence> {
    if sequence.is_empty() || sequence.len() < 100 {
        return None;
    }

    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let hla_id = parts[0].to_string();
    let allele_name = parts[1].to_string();

    // Extract locus from allele name (e.g., "A*01:01:01:01" -> "A")
    let locus = allele_name.split('*').next()
        .unwrap_or("unknown")
        .to_string();

    Some(HlaAlleleSequence {
        hla_id,
        allele_name,
        locus,
        sequence: sequence.to_uppercase(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bold_header() {
        let header = "ANICT932-11|Danaus plexippus|COI-5P|KF398175";
        let seq = "ATCGATCG".repeat(100);
        let result = parse_bold_header(header, &seq).unwrap();

        assert_eq!(result.id, "ANICT932-11");
        assert_eq!(result.species, "Danaus plexippus");
        assert_eq!(result.marker, "COI-5P");
        assert_eq!(result.accession, Some("KF398175".to_string()));
    }

    #[test]
    fn test_parse_hla_header() {
        let header = "HLA:HLA00001 A*01:01:01:01 1098 bp";
        let seq = "ATCGATCG".repeat(100);
        let result = parse_hla_header(header, &seq).unwrap();

        assert_eq!(result.hla_id, "HLA:HLA00001");
        assert_eq!(result.allele_name, "A*01:01:01:01");
        assert_eq!(result.locus, "A");
    }
}
