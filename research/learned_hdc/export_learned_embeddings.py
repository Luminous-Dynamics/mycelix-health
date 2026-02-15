#!/usr/bin/env python3
"""
Export Learned HDC Embeddings for Rust Integration

This script trains the hybrid HDC model and exports the learned k-mer embeddings
in a JSON format that can be loaded by the Rust hdc-core library.

The export format is:
{
  "kmer_length": 6,
  "dimension": 1000,
  "embeddings": {
    "ACGTAC": [0.5, -0.3, ...],
    "CGTACG": [-0.2, 0.8, ...]
  }
}
"""

import json
import os
import sys
from typing import List, Dict

# Add parent directory for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from hybrid_hdc_combined import HybridHDCModel, HybridConfig


def train_and_export(
    sequences: List[str],
    labels: List[int],
    output_path: str,
    config: HybridConfig = None,
    seed: int = 42
) -> Dict:
    """
    Train hybrid HDC model and export learned k-mer embeddings.

    Args:
        sequences: List of DNA sequences for training
        labels: Binary labels (0/1) for sequences
        output_path: Path to save JSON embeddings
        config: Optional HybridConfig (uses defaults if None)
        seed: Random seed for reproducibility

    Returns:
        Dictionary with training stats and export info
    """
    if config is None:
        config = HybridConfig(
            dim=1000,
            kmer_length=6,
            num_classes=2,
            contrastive_epochs=15,
            finetune_epochs=40,
            patience=5
        )

    print(f"Training Hybrid HDC model...")
    print(f"  Dimension: {config.dim}")
    print(f"  K-mer length: {config.kmer_length}")
    print(f"  Sequences: {len(sequences)}")

    # Create and train model
    model = HybridHDCModel(config, seed=seed)

    # Pre-train using contrastive learning
    print("\nContrastive pre-training...")
    model.contrastive_pretrain(sequences, verbose=True)

    # Split for fine-tuning
    split = int(0.8 * len(sequences))
    train_seqs, train_labels = sequences[:split], labels[:split]
    val_seqs, val_labels = sequences[split:], labels[split:]

    # Fine-tune
    print("\nFine-tuning...")
    model.finetune(train_seqs, train_labels, val_seqs, val_labels, verbose=True)

    # Export embeddings
    print(f"\nExporting embeddings to {output_path}...")
    embeddings = {}

    for kmer, idx in model.kmer_to_idx.items():
        # Get the learned embedding for this k-mer
        embedding = model.embeddings[idx].tolist()
        embeddings[kmer] = embedding

    export_data = {
        "kmer_length": config.kmer_length,
        "dimension": config.dim,
        "embeddings": embeddings
    }

    with open(output_path, 'w') as f:
        json.dump(export_data, f)

    # Export MLP classifier weights
    if model.W1 is not None:
        mlp_path = output_path.replace('.json', '_mlp.json')
        print(f"Exporting MLP classifier to {mlp_path}...")

        mlp_data = {
            "input_dim": config.dim,
            "hidden_dim": config.mlp_hidden,
            "output_dim": config.num_classes,
            "W1": model.W1.tolist(),
            "b1": model.b1.tolist(),
            "W2": model.W2.tolist(),
            "b2": model.b2.tolist(),
        }

        with open(mlp_path, 'w') as f:
            json.dump(mlp_data, f)

        mlp_size = os.path.getsize(mlp_path)
        print(f"  MLP exported: {mlp_size / 1024:.1f} KB")

    # Calculate file size
    file_size = os.path.getsize(output_path)

    print(f"  K-mers exported: {len(embeddings)}")
    print(f"  File size: {file_size / 1024 / 1024:.1f} MB")

    return {
        "num_kmers": len(embeddings),
        "dimension": config.dim,
        "kmer_length": config.kmer_length,
        "file_size_bytes": file_size,
        "output_path": output_path
    }


def export_compact(
    sequences: List[str],
    labels: List[int],
    output_path: str,
    config: HybridConfig = None,
    seed: int = 42
) -> Dict:
    """
    Export embeddings in a more compact binary format.

    Uses numpy's save format for smaller file sizes.
    """
    import numpy as np

    if config is None:
        config = HybridConfig(dim=1000, kmer_length=6)

    print(f"Training for compact export...")
    model = HybridHDCModel(config, seed=seed)
    model.contrastive_pretrain(sequences, verbose=True)

    split = int(0.8 * len(sequences))
    model.finetune(
        sequences[:split], labels[:split],
        sequences[split:], labels[split:],
        verbose=True
    )

    # Export as numpy file
    np_path = output_path.replace('.json', '.npz')

    kmer_list = list(model.kmer_to_idx.keys())
    embeddings = np.array([model.embeddings[model.kmer_to_idx[k]] for k in kmer_list])

    np.savez_compressed(
        np_path,
        kmers=np.array(kmer_list),
        embeddings=embeddings,
        kmer_length=config.kmer_length,
        dimension=config.dim
    )

    file_size = os.path.getsize(np_path)
    print(f"  Compact export: {np_path}")
    print(f"  File size: {file_size / 1024 / 1024:.1f} MB")

    return {
        "num_kmers": len(kmer_list),
        "dimension": config.dim,
        "file_size_bytes": file_size,
        "output_path": np_path
    }


def main():
    """Train on sample data and export."""
    import numpy as np

    # Generate sample training data
    np.random.seed(42)
    bases = ['A', 'C', 'G', 'T']

    # Create sequences with a learnable pattern
    sequences = []
    labels = []

    # Positive class: contains TATAAA motif
    for _ in range(500):
        seq = ''.join(np.random.choice(bases) for _ in range(100))
        # Insert TATA box at random position
        pos = np.random.randint(20, 80)
        seq = seq[:pos] + 'TATAAA' + seq[pos+6:]
        sequences.append(seq)
        labels.append(1)

    # Negative class: random sequences without TATA
    for _ in range(500):
        seq = ''.join(np.random.choice(bases) for _ in range(100))
        # Ensure no TATA box
        seq = seq.replace('TATAAA', 'GGGGGG')
        sequences.append(seq)
        labels.append(0)

    # Train and export
    config = HybridConfig(
        dim=1000,
        kmer_length=6,
        num_classes=2,
        contrastive_epochs=10,
        finetune_epochs=20,
        patience=5
    )

    output_dir = os.path.dirname(os.path.abspath(__file__))
    output_path = os.path.join(output_dir, 'models', 'learned_6mers.json')

    # Create models directory
    os.makedirs(os.path.join(output_dir, 'models'), exist_ok=True)

    result = train_and_export(sequences, labels, output_path, config)

    print("\n" + "="*60)
    print("EXPORT COMPLETE")
    print("="*60)
    print(f"  Output: {result['output_path']}")
    print(f"  K-mers: {result['num_kmers']}")
    print(f"  Dimension: {result['dimension']}")
    print(f"  Size: {result['file_size_bytes'] / 1024 / 1024:.1f} MB")
    print()
    print("To use in Rust:")
    print('  let codebook = LearnedKmerCodebook::load("models/learned_6mers.json")?;')
    print('  let encoded = encoder.encode_with_learned_codebook(sequence, &codebook)?;')


if __name__ == '__main__':
    main()
