#!/usr/bin/env python3
"""
Hyperbolic HDC for Phylogenetics

Embeds DNA sequences in hyperbolic space (Poincaré ball model).
Hyperbolic geometry naturally represents tree-like hierarchies.

Key insight: Phylogenetic trees are hierarchical - species branch
from common ancestors. Euclidean space struggles with trees,
but hyperbolic space has exponentially more "room" for branches.

Applications:
- Taxonomic classification
- Ancestral sequence prediction
- Phylogenetic placement
"""

import numpy as np
from typing import List, Tuple, Dict
from dataclasses import dataclass
import time
from collections import Counter

HYPERVECTOR_DIM = 500  # Smaller for hyperbolic (geometric constraints)


@dataclass
class HyperbolicConfig:
    dim: int = HYPERVECTOR_DIM
    kmer_length: int = 6
    num_classes: int = 10  # For taxonomy
    learning_rate: float = 0.01
    curvature: float = 1.0  # Hyperbolic curvature (1.0 = standard)
    max_norm: float = 0.95  # Keep vectors inside Poincaré ball


def generate_kmers(k: int) -> List[str]:
    bases = ['A', 'C', 'G', 'T']
    if k == 1:
        return bases
    smaller = generate_kmers(k - 1)
    return [b + s for b in bases for s in smaller]


# ============================================================================
# HYPERBOLIC GEOMETRY OPERATIONS (Poincaré Ball Model)
# ============================================================================

def poincare_add(x: np.ndarray, y: np.ndarray, c: float = 1.0) -> np.ndarray:
    """
    Möbius addition in Poincaré ball.

    x ⊕ y = ((1 + 2c<x,y> + c||y||²)x + (1 - c||x||²)y) /
            (1 + 2c<x,y> + c²||x||²||y||²)
    """
    xy = np.dot(x, y)
    x_sq = np.dot(x, x)
    y_sq = np.dot(y, y)

    num = (1 + 2*c*xy + c*y_sq) * x + (1 - c*x_sq) * y
    denom = 1 + 2*c*xy + c*c*x_sq*y_sq

    return num / (denom + 1e-10)


def poincare_distance(x: np.ndarray, y: np.ndarray, c: float = 1.0) -> float:
    """
    Hyperbolic distance in Poincaré ball.

    d(x,y) = (2/sqrt(c)) * arctanh(sqrt(c) * ||−x ⊕ y||)
    """
    diff = poincare_add(-x, y, c)
    norm_diff = np.linalg.norm(diff)

    # Clamp for numerical stability
    sqrt_c = np.sqrt(c)
    arg = np.clip(sqrt_c * norm_diff, -1 + 1e-7, 1 - 1e-7)

    return (2 / sqrt_c) * np.arctanh(arg)


def project_to_ball(x: np.ndarray, max_norm: float = 0.95) -> np.ndarray:
    """Project vector to inside Poincaré ball."""
    norm = np.linalg.norm(x)
    if norm > max_norm:
        return x * (max_norm / norm)
    return x


def exp_map(v: np.ndarray, x: np.ndarray, c: float = 1.0) -> np.ndarray:
    """
    Exponential map: tangent space -> manifold.

    exp_x(v) = x ⊕ (tanh(sqrt(c) * λ_x * ||v|| / 2) * v / (sqrt(c) * ||v||))

    where λ_x = 2 / (1 - c||x||²) is the conformal factor.
    """
    norm_v = np.linalg.norm(v)
    if norm_v < 1e-10:
        return x

    x_sq = np.dot(x, x)
    lambda_x = 2 / (1 - c * x_sq + 1e-10)

    sqrt_c = np.sqrt(c)
    arg = sqrt_c * lambda_x * norm_v / 2
    scale = np.tanh(np.clip(arg, -15, 15)) / (sqrt_c * norm_v)

    return poincare_add(x, scale * v, c)


def log_map(y: np.ndarray, x: np.ndarray, c: float = 1.0) -> np.ndarray:
    """
    Logarithmic map: manifold -> tangent space.

    log_x(y) = (2 / (sqrt(c) * λ_x)) * arctanh(sqrt(c) * ||−x ⊕ y||) * (−x ⊕ y) / ||−x ⊕ y||
    """
    diff = poincare_add(-x, y, c)
    norm_diff = np.linalg.norm(diff)

    if norm_diff < 1e-10:
        return np.zeros_like(x)

    x_sq = np.dot(x, x)
    lambda_x = 2 / (1 - c * x_sq + 1e-10)

    sqrt_c = np.sqrt(c)
    arg = np.clip(sqrt_c * norm_diff, -1 + 1e-7, 1 - 1e-7)
    scale = (2 / (sqrt_c * lambda_x)) * np.arctanh(arg) / norm_diff

    return scale * diff


class HyperbolicHDCEncoder:
    """
    HDC encoder in hyperbolic space.

    K-mer embeddings live in Poincaré ball.
    Sequence encoding uses Möbius operations.
    """

    def __init__(self, config: HyperbolicConfig, seed: int = 42):
        self.config = config
        self.rng = np.random.RandomState(seed)

        num_kmers = 4 ** config.kmer_length

        # Initialize k-mer embeddings in Poincaré ball
        # Start near origin (small norm) for numerical stability
        raw = self.rng.randn(num_kmers, config.dim) * 0.01
        self.embeddings = np.array([project_to_ball(v, config.max_norm * 0.5)
                                    for v in raw])

        self.kmers = generate_kmers(config.kmer_length)
        self.kmer_to_idx = {kmer: i for i, kmer in enumerate(self.kmers)}

        # Class prototypes (for classification)
        raw_proto = self.rng.randn(config.num_classes, config.dim) * 0.01
        self.prototypes = np.array([project_to_ball(v, config.max_norm * 0.5)
                                     for v in raw_proto])

    def encode(self, sequence: str) -> np.ndarray:
        """
        Encode sequence in hyperbolic space.

        Uses Möbius midpoint (Fréchet mean) of k-mer embeddings.
        """
        k = self.config.kmer_length
        c = self.config.curvature

        if len(sequence) < k:
            return np.zeros(self.config.dim)

        indices = []
        for i in range(len(sequence) - k + 1):
            kmer = sequence[i:i+k].upper()
            idx = self.kmer_to_idx.get(kmer)
            if idx is not None:
                indices.append(idx)

        if not indices:
            return np.zeros(self.config.dim)

        # Hyperbolic averaging: iterative weighted midpoint
        # Start from first embedding
        result = self.embeddings[indices[0]].copy()

        for i, idx in enumerate(indices[1:], 1):
            # Weight for averaging: 1/(i+1)
            weight = 1 / (i + 1)

            # Move toward new point via geodesic
            direction = log_map(self.embeddings[idx], result, c)
            result = exp_map(weight * direction, result, c)
            result = project_to_ball(result, self.config.max_norm)

        return result

    def distance_to_prototype(self, encoding: np.ndarray, class_id: int) -> float:
        """Hyperbolic distance to class prototype."""
        return poincare_distance(encoding, self.prototypes[class_id], self.config.curvature)

    def predict(self, sequence: str) -> int:
        """Predict class based on nearest prototype."""
        encoding = self.encode(sequence)
        distances = [self.distance_to_prototype(encoding, i)
                     for i in range(self.config.num_classes)]
        return np.argmin(distances)

    def train_step(self, sequences: List[str], labels: List[int]) -> float:
        """
        Training step: move prototypes toward/away from samples.

        Uses Riemannian SGD in hyperbolic space.
        """
        c = self.config.curvature
        lr = self.config.learning_rate

        total_loss = 0

        for seq, label in zip(sequences, labels):
            encoding = self.encode(seq)

            # Compute distances to all prototypes
            distances = [poincare_distance(encoding, self.prototypes[i], c)
                         for i in range(self.config.num_classes)]

            # Softmin loss
            neg_distances = [-d for d in distances]
            exp_neg = np.exp(np.clip(neg_distances, -20, 20))
            probs = exp_neg / (exp_neg.sum() + 1e-10)

            # Loss: negative log probability of correct class
            loss = -np.log(probs[label] + 1e-10)
            total_loss += loss

            # Update prototypes via Riemannian SGD
            for i in range(self.config.num_classes):
                # Gradient direction (in tangent space)
                direction = log_map(encoding, self.prototypes[i], c)

                if i == label:
                    # Move toward correct class
                    grad = direction * (1 - probs[i])
                else:
                    # Move away from wrong classes
                    grad = -direction * probs[i]

                # Update via exponential map
                new_proto = exp_map(lr * grad, self.prototypes[i], c)
                self.prototypes[i] = project_to_ball(new_proto, self.config.max_norm)

        return total_loss / len(sequences)


class EuclideanHDCEncoder:
    """Euclidean baseline for comparison."""

    def __init__(self, config: HyperbolicConfig, seed: int = 42):
        self.config = config
        self.rng = np.random.RandomState(seed)

        num_kmers = 4 ** config.kmer_length
        self.embeddings = self.rng.randn(num_kmers, config.dim) * 0.1

        self.kmers = generate_kmers(config.kmer_length)
        self.kmer_to_idx = {kmer: i for i, kmer in enumerate(self.kmers)}

        self.prototypes = self.rng.randn(config.num_classes, config.dim) * 0.1

    def encode(self, sequence: str) -> np.ndarray:
        k = self.config.kmer_length
        if len(sequence) < k:
            return np.zeros(self.config.dim)

        indices = []
        for i in range(len(sequence) - k + 1):
            kmer = sequence[i:i+k].upper()
            idx = self.kmer_to_idx.get(kmer)
            if idx is not None:
                indices.append(idx)

        if not indices:
            return np.zeros(self.config.dim)

        return self.embeddings[indices].mean(axis=0)

    def predict(self, sequence: str) -> int:
        encoding = self.encode(sequence)
        distances = [np.linalg.norm(encoding - self.prototypes[i])
                     for i in range(self.config.num_classes)]
        return np.argmin(distances)

    def train_step(self, sequences: List[str], labels: List[int]) -> float:
        lr = self.config.learning_rate
        total_loss = 0

        for seq, label in zip(sequences, labels):
            encoding = self.encode(seq)

            distances = [np.linalg.norm(encoding - self.prototypes[i])
                         for i in range(self.config.num_classes)]

            neg_distances = [-d for d in distances]
            exp_neg = np.exp(np.clip(neg_distances, -20, 20))
            probs = exp_neg / (exp_neg.sum() + 1e-10)

            loss = -np.log(probs[label] + 1e-10)
            total_loss += loss

            for i in range(self.config.num_classes):
                direction = encoding - self.prototypes[i]
                direction = direction / (np.linalg.norm(direction) + 1e-10)

                if i == label:
                    self.prototypes[i] += lr * direction * (1 - probs[i])
                else:
                    self.prototypes[i] -= lr * direction * probs[i]

        return total_loss / len(sequences)


def generate_phylogenetic_dataset(n_species: int, n_per_species: int,
                                   seq_length: int = 150, seed: int = 42):
    """
    Generate synthetic phylogenetic dataset with tree structure.

    Creates hierarchical species relationships:
    - Species 0-2: Clade A (share common ancestor)
    - Species 3-5: Clade B
    - Species 6-9: Clade C

    This tests whether hyperbolic embeddings can capture hierarchy.
    """
    rng = np.random.RandomState(seed)
    bases = ['A', 'C', 'G', 'T']
    sequences, labels = [], []

    # Create ancestral sequences for each clade
    ancestors = {
        'root': ''.join(rng.choice(bases, seq_length)),
    }

    # Clade ancestors (diverged from root)
    for clade, divergence in [('A', 0.1), ('B', 0.15), ('C', 0.2)]:
        seq = list(ancestors['root'])
        for i in range(len(seq)):
            if rng.random() < divergence:
                seq[i] = rng.choice(bases)
        ancestors[clade] = ''.join(seq)

    # Assign species to clades
    species_clades = {
        0: 'A', 1: 'A', 2: 'A',
        3: 'B', 4: 'B', 5: 'B',
        6: 'C', 7: 'C', 8: 'C', 9: 'C'
    }

    # Create species sequences (diverged from clade ancestor)
    species_refs = {}
    for species_id, clade in species_clades.items():
        seq = list(ancestors[clade])
        # Each species diverges from clade ancestor
        species_divergence = 0.05 + rng.random() * 0.05
        for i in range(len(seq)):
            if rng.random() < species_divergence:
                seq[i] = rng.choice(bases)
        species_refs[species_id] = ''.join(seq)

    # Generate samples for each species
    for species_id in range(n_species):
        ref = species_refs[species_id]
        for _ in range(n_per_species):
            seq = list(ref)
            # Within-species variation
            for i in range(len(seq)):
                if rng.random() < 0.02:  # Low within-species variation
                    seq[i] = rng.choice(bases)
            sequences.append(''.join(seq))
            labels.append(species_id)

    idx = rng.permutation(len(sequences))
    return [sequences[i] for i in idx], [labels[i] for i in idx]


def run_experiment(n_species: int = 10, n_per_species: int = 50, epochs: int = 30):
    """Run hyperbolic vs Euclidean comparison."""
    print("\n" + "="*70)
    print("  HYPERBOLIC HDC FOR PHYLOGENETICS")
    print("  Comparing Euclidean vs Hyperbolic embeddings")
    print("="*70 + "\n", flush=True)

    # Generate phylogenetic dataset
    print("Generating phylogenetic dataset with tree structure...", flush=True)
    sequences, labels = generate_phylogenetic_dataset(n_species, n_per_species)

    split = int(0.8 * len(sequences))
    train_seqs, test_seqs = sequences[:split], sequences[split:]
    train_labels, test_labels = labels[:split], labels[split:]

    print(f"  Train: {len(train_seqs)}, Test: {len(test_seqs)}")
    print(f"  Species: {n_species} (organized in 3 clades)", flush=True)

    config = HyperbolicConfig(dim=HYPERVECTOR_DIM, kmer_length=6, num_classes=n_species)

    # Euclidean baseline
    print("\n" + "-"*50)
    print("EUCLIDEAN HDC (Prototype classifier)")
    print("-"*50, flush=True)

    euclidean = EuclideanHDCEncoder(config)

    t0 = time.time()
    for epoch in range(epochs):
        idx = np.random.permutation(len(train_seqs))
        train_seqs_shuf = [train_seqs[i] for i in idx]
        train_labels_shuf = [train_labels[i] for i in idx]

        loss = euclidean.train_step(train_seqs_shuf, train_labels_shuf)

        if (epoch + 1) % 10 == 0:
            preds = [euclidean.predict(seq) for seq in test_seqs]
            acc = np.mean(np.array(preds) == np.array(test_labels))
            print(f"  Epoch {epoch+1}: Loss={loss:.4f}, Test Acc={acc*100:.2f}%", flush=True)

    euclidean_time = time.time() - t0
    preds = [euclidean.predict(seq) for seq in test_seqs]
    euclidean_acc = np.mean(np.array(preds) == np.array(test_labels))
    print(f"  Final Accuracy: {euclidean_acc*100:.2f}%")
    print(f"  Training Time: {euclidean_time:.2f}s", flush=True)

    # Hyperbolic
    print("\n" + "-"*50)
    print("HYPERBOLIC HDC (Poincaré ball)")
    print(f"  Curvature: {config.curvature}")
    print("-"*50, flush=True)

    hyperbolic = HyperbolicHDCEncoder(config)

    t0 = time.time()
    for epoch in range(epochs):
        idx = np.random.permutation(len(train_seqs))
        train_seqs_shuf = [train_seqs[i] for i in idx]
        train_labels_shuf = [train_labels[i] for i in idx]

        loss = hyperbolic.train_step(train_seqs_shuf, train_labels_shuf)

        if (epoch + 1) % 10 == 0:
            preds = [hyperbolic.predict(seq) for seq in test_seqs]
            acc = np.mean(np.array(preds) == np.array(test_labels))
            print(f"  Epoch {epoch+1}: Loss={loss:.4f}, Test Acc={acc*100:.2f}%", flush=True)

    hyperbolic_time = time.time() - t0
    preds = [hyperbolic.predict(seq) for seq in test_seqs]
    hyperbolic_acc = np.mean(np.array(preds) == np.array(test_labels))
    print(f"  Final Accuracy: {hyperbolic_acc*100:.2f}%")
    print(f"  Training Time: {hyperbolic_time:.2f}s", flush=True)

    # Analyze hierarchical structure
    print("\n" + "-"*50)
    print("CLADE ANALYSIS")
    print("-"*50, flush=True)

    # Check if prototypes reflect clade structure
    clades = {0: [0,1,2], 1: [3,4,5], 2: [6,7,8,9]}

    print("  Euclidean inter-clade distances:")
    for c1 in range(3):
        for c2 in range(c1+1, 3):
            dists = []
            for s1 in clades[c1]:
                for s2 in clades[c2]:
                    d = np.linalg.norm(euclidean.prototypes[s1] - euclidean.prototypes[s2])
                    dists.append(d)
            print(f"    Clade {c1} vs Clade {c2}: {np.mean(dists):.3f}", flush=True)

    print("\n  Hyperbolic inter-clade distances:")
    for c1 in range(3):
        for c2 in range(c1+1, 3):
            dists = []
            for s1 in clades[c1]:
                for s2 in clades[c2]:
                    d = poincare_distance(hyperbolic.prototypes[s1],
                                          hyperbolic.prototypes[s2],
                                          config.curvature)
                    dists.append(d)
            print(f"    Clade {c1} vs Clade {c2}: {np.mean(dists):.3f}", flush=True)

    # Summary
    print("\n" + "="*70)
    print("SUMMARY")
    print("="*70)
    print(f"\n  {'Method':<40} {'Accuracy':>12}")
    print(f"  {'-'*52}")
    print(f"  {'Euclidean HDC':<40} {euclidean_acc*100:>11.2f}%")
    print(f"  {'Hyperbolic HDC':<40} {hyperbolic_acc*100:>11.2f}%")

    improvement = (hyperbolic_acc - euclidean_acc) / euclidean_acc * 100 if euclidean_acc > 0 else 0
    print(f"\n  Hyperbolic Improvement: {improvement:+.1f}%", flush=True)

    return {
        'euclidean_acc': euclidean_acc,
        'hyperbolic_acc': hyperbolic_acc,
        'improvement': improvement
    }


if __name__ == '__main__':
    print("\n" + "="*70)
    print("  HYPERBOLIC HDC FOR PHYLOGENETICS")
    print("  Tree-structured data in hyperbolic space")
    print("="*70, flush=True)

    results = run_experiment(n_species=10, n_per_species=50, epochs=30)

    print("\n" + "="*70)
    print("KEY INSIGHTS:")
    print("="*70)
    print(f"""
  1. Hyperbolic space naturally represents hierarchical relationships
  2. Poincaré ball model has infinite space near boundary
  3. Improvement over Euclidean: {results['improvement']:+.1f}%

  WHY HYPERBOLIC FOR PHYLOGENETICS:
  - Evolutionary trees are inherently hierarchical
  - Euclidean space has limited capacity for tree branches
  - Hyperbolic: volume grows EXPONENTIALLY with radius
  - Better separation of species within vs between clades

  APPLICATIONS:
  - Taxonomic classification (species/genus/family)
  - Ancestral sequence reconstruction
  - Phylogenetic placement of novel sequences
  - Metagenomic binning
""", flush=True)
