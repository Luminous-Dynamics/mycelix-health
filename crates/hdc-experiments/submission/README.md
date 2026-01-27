# Publication Submission Files

## Submission Status

**Target**: bioRxiv (Bioinformatics section)

**Title**: Hyperdimensional Computing for Privacy-Preserving Genetic Similarity

**Authors**: Luminous Dynamics Team

## Files

- `manuscript.md` - Full manuscript in Markdown format (bioRxiv accepts Markdown)
- `figures/` - High-resolution figures (if any)
- `supplementary/` - Supplementary materials

## Submission Checklist

- [x] Abstract (150-250 words)
- [x] Introduction with clear problem statement
- [x] Methods with reproducible details
- [x] Results with statistical validation
- [x] Discussion comparing to prior work
- [x] Data availability statement
- [x] Code availability (GitHub link)
- [x] Author contributions
- [x] Competing interests declaration
- [x] Funding statement

## Author Contributions

- **Tristan Stoltz**: Conceptualization, methodology, software development, validation, writing
- **Claude (AI Assistant)**: Code implementation, documentation, experimental analysis

## Competing Interests

The authors declare no competing interests.

## Funding

This work was conducted as part of the Mycelix-Health project. No external funding was received.

## Data Sources

- **BOLD Systems**: https://boldsystems.org/ (COI barcode sequences)
- **IMGT/HLA**: https://github.com/ANHIG/IMGTHLA (HLA reference alleles)
- **NCBI RefSeq**: CYP450 reference sequences

## Code Repository

https://github.com/Luminous-Dynamics/mycelix-health

## Key Results Summary

| Experiment | Accuracy | N |
|------------|----------|---|
| Taxonomy (COI) | 89.3% order classification | 272 sequences |
| HLA (IMGT) | 100% locus classification | 300 alleles |
| Privacy (attacks) | LOW risk membership inference | 100 sequences |
| Performance | ~2.2µs similarity (SIMD) | 100K iterations |

## bioRxiv Submission Steps

1. Create account at https://www.biorxiv.org/
2. Start new submission
3. Select "Bioinformatics" as subject area
4. Upload manuscript.md or PDF
5. Add metadata (title, abstract, authors)
6. Submit for screening
7. Receive DOI within 24-48 hours
