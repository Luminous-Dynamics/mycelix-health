"""
Learned Hyperdimensional Computing for DNA Sequences

This module implements trainable k-mer embeddings that maintain HDC's
fast inference while achieving higher accuracy through gradient-based learning.

Key insight: Instead of random k-mer vectors, we learn them to optimize
task performance while keeping the same inference pipeline (XOR + popcount).
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
import numpy as np
from typing import List, Tuple, Optional, Dict
from dataclasses import dataclass
import time


# Hypervector dimension (matches Rust implementation)
HYPERVECTOR_DIM = 10_000


@dataclass
class HDCConfig:
    """Configuration for HDC models."""
    dim: int = HYPERVECTOR_DIM
    kmer_length: int = 6
    num_classes: int = 2
    use_position_encoding: bool = True
    binarize_inference: bool = True
    temperature: float = 1.0  # For soft binarization during training


def generate_kmers(k: int) -> List[str]:
    """Generate all possible k-mers."""
    bases = ['A', 'C', 'G', 'T']
    if k == 1:
        return bases
    smaller = generate_kmers(k - 1)
    return [b + s for b in bases for s in smaller]


class LearnedItemMemory(nn.Module):
    """
    Learned item memory for k-mers.

    Instead of random vectors, we learn optimal embeddings via gradient descent.
    During inference, we can binarize for fast XOR + popcount operations.
    """

    def __init__(self, config: HDCConfig):
        super().__init__()
        self.config = config

        # Number of possible k-mers (4^k)
        num_kmers = 4 ** config.kmer_length

        # Learnable continuous embeddings
        # Initialize with values that will binarize to ~50% ones
        self.embeddings = nn.Parameter(
            torch.randn(num_kmers, config.dim) * 0.1
        )

        # Position encodings (learnable permutation-like vectors)
        if config.use_position_encoding:
            # Max sequence length we'll handle
            max_positions = 1000
            self.position_embeddings = nn.Parameter(
                torch.randn(max_positions, config.dim) * 0.1
            )

        # Build k-mer to index mapping
        self.kmers = generate_kmers(config.kmer_length)
        self.kmer_to_idx = {kmer: i for i, kmer in enumerate(self.kmers)}

    def get_kmer_vector(self, kmer: str, position: int = 0) -> torch.Tensor:
        """Get the (soft) vector for a k-mer at a position."""
        idx = self.kmer_to_idx.get(kmer)
        if idx is None:
            # Unknown k-mer (contains N or other characters)
            return torch.zeros(self.config.dim, device=self.embeddings.device)

        vec = self.embeddings[idx]

        if self.config.use_position_encoding and position < len(self.position_embeddings):
            # Add position encoding (like binding in HDC)
            vec = vec * torch.sigmoid(self.position_embeddings[position])

        return vec

    def binarize(self, vec: torch.Tensor) -> torch.Tensor:
        """Convert continuous vector to binary."""
        if self.training:
            # Soft binarization during training (straight-through estimator)
            hard = (vec > 0).float()
            soft = torch.sigmoid(vec / self.config.temperature)
            return hard - soft.detach() + soft
        else:
            return (vec > 0).float()


class LearnedHDCEncoder(nn.Module):
    """
    DNA sequence encoder using learned HDC.

    Encodes sequences by:
    1. Extracting k-mers
    2. Looking up learned embeddings
    3. Bundling with position information
    4. Optionally binarizing for fast inference
    """

    def __init__(self, config: HDCConfig):
        super().__init__()
        self.config = config
        self.item_memory = LearnedItemMemory(config)

    def encode_sequence(self, sequence: str) -> torch.Tensor:
        """Encode a DNA sequence to a hypervector."""
        k = self.config.kmer_length

        if len(sequence) < k:
            return torch.zeros(self.config.dim, device=self.item_memory.embeddings.device)

        # Extract k-mers and their positions
        kmer_vectors = []
        for i in range(len(sequence) - k + 1):
            kmer = sequence[i:i+k].upper()
            vec = self.item_memory.get_kmer_vector(kmer, position=i)
            kmer_vectors.append(vec)

        if not kmer_vectors:
            return torch.zeros(self.config.dim, device=self.item_memory.embeddings.device)

        # Bundle: average then threshold (majority vote approximation)
        stacked = torch.stack(kmer_vectors)
        bundled = stacked.mean(dim=0)

        # Binarize
        binary = self.item_memory.binarize(bundled)

        return binary

    def encode_batch(self, sequences: List[str]) -> torch.Tensor:
        """Encode multiple sequences."""
        vectors = [self.encode_sequence(seq) for seq in sequences]
        return torch.stack(vectors)

    def similarity(self, vec1: torch.Tensor, vec2: torch.Tensor) -> torch.Tensor:
        """Compute similarity between two vectors."""
        if self.config.binarize_inference:
            # Hamming similarity for binary vectors
            matches = (vec1 == vec2).float().sum(dim=-1)
            return matches / self.config.dim
        else:
            # Cosine similarity for continuous
            return F.cosine_similarity(vec1, vec2, dim=-1)


class LearnedHDCClassifier(nn.Module):
    """
    Full classification model using learned HDC.

    Architecture:
    1. LearnedHDCEncoder encodes sequences
    2. Optional lightweight refinement head
    3. Classification output
    """

    def __init__(self, config: HDCConfig, use_refinement_head: bool = True):
        super().__init__()
        self.config = config
        self.encoder = LearnedHDCEncoder(config)

        # Optional refinement head (small MLP)
        if use_refinement_head:
            self.refinement = nn.Sequential(
                nn.Linear(config.dim, 256),
                nn.ReLU(),
                nn.Dropout(0.1),
                nn.Linear(256, 64),
                nn.ReLU(),
            )
            self.classifier = nn.Linear(64, config.num_classes)
        else:
            self.refinement = None
            self.classifier = nn.Linear(config.dim, config.num_classes)

    def forward(self, sequences: List[str]) -> torch.Tensor:
        """Forward pass for classification."""
        # Encode sequences
        encoded = self.encoder.encode_batch(sequences)

        # Refinement
        if self.refinement is not None:
            features = self.refinement(encoded)
        else:
            features = encoded

        # Classify
        logits = self.classifier(features)
        return logits

    def get_embeddings(self, sequences: List[str]) -> torch.Tensor:
        """Get HDC embeddings without classification."""
        return self.encoder.encode_batch(sequences)


class BaselineHDCEncoder:
    """
    Baseline HDC encoder with random k-mer vectors (for comparison).
    """

    def __init__(self, config: HDCConfig, seed: int = 42):
        self.config = config
        self.rng = np.random.RandomState(seed)

        # Generate random binary vectors for each k-mer
        num_kmers = 4 ** config.kmer_length
        self.embeddings = self.rng.randint(0, 2, size=(num_kmers, config.dim)).astype(np.float32)

        # Position permutation patterns
        self.position_perms = []
        for _ in range(1000):
            perm = self.rng.permutation(config.dim)
            self.position_perms.append(perm)

        # K-mer mapping
        self.kmers = generate_kmers(config.kmer_length)
        self.kmer_to_idx = {kmer: i for i, kmer in enumerate(self.kmers)}

    def encode_sequence(self, sequence: str) -> np.ndarray:
        """Encode a DNA sequence."""
        k = self.config.kmer_length

        if len(sequence) < k:
            return np.zeros(self.config.dim, dtype=np.float32)

        # Accumulate k-mer contributions
        accumulator = np.zeros(self.config.dim, dtype=np.float32)
        count = 0

        for i in range(len(sequence) - k + 1):
            kmer = sequence[i:i+k].upper()
            idx = self.kmer_to_idx.get(kmer)
            if idx is not None:
                vec = self.embeddings[idx]
                # Apply position permutation
                if self.config.use_position_encoding and i < len(self.position_perms):
                    vec = vec[self.position_perms[i]]
                accumulator += vec
                count += 1

        if count == 0:
            return np.zeros(self.config.dim, dtype=np.float32)

        # Majority vote
        return (accumulator > count / 2).astype(np.float32)

    def encode_batch(self, sequences: List[str]) -> np.ndarray:
        """Encode multiple sequences."""
        return np.stack([self.encode_sequence(seq) for seq in sequences])

    def similarity(self, vec1: np.ndarray, vec2: np.ndarray) -> float:
        """Hamming similarity."""
        return np.mean(vec1 == vec2)


# ============================================================================
# Dataset Generation
# ============================================================================

def generate_promoter_dataset(
    n_samples: int = 1000,
    seq_length: int = 100,
    seed: int = 42
) -> Tuple[List[str], List[int]]:
    """
    Generate synthetic promoter classification dataset.

    Promoters have specific motifs (TATA box, GC box, etc.)
    Non-promoters are random sequences.
    """
    rng = np.random.RandomState(seed)
    bases = ['A', 'C', 'G', 'T']

    sequences = []
    labels = []

    # Promoter motifs
    tata_box = "TATAAA"
    gc_box = "GGGCGG"
    caat_box = "CCAAT"

    for i in range(n_samples):
        if i < n_samples // 2:
            # Promoter sequence
            seq = list(''.join(rng.choice(bases, seq_length)))

            # Insert TATA box around position 25-30
            pos = rng.randint(20, 35)
            if pos + len(tata_box) < seq_length:
                for j, c in enumerate(tata_box):
                    seq[pos + j] = c

            # Maybe add GC box
            if rng.random() > 0.3:
                pos = rng.randint(40, 60)
                if pos + len(gc_box) < seq_length:
                    for j, c in enumerate(gc_box):
                        seq[pos + j] = c

            sequences.append(''.join(seq))
            labels.append(1)
        else:
            # Non-promoter (random)
            seq = ''.join(rng.choice(bases, seq_length))
            sequences.append(seq)
            labels.append(0)

    # Shuffle
    indices = rng.permutation(len(sequences))
    sequences = [sequences[i] for i in indices]
    labels = [labels[i] for i in indices]

    return sequences, labels


def generate_taxonomy_dataset(
    n_species: int = 10,
    n_samples_per_species: int = 50,
    seq_length: int = 200,
    mutation_rate: float = 0.05,
    seed: int = 42
) -> Tuple[List[str], List[int]]:
    """
    Generate synthetic taxonomy classification dataset.

    Each species has a reference sequence.
    Samples are mutations of the reference.
    """
    rng = np.random.RandomState(seed)
    bases = ['A', 'C', 'G', 'T']

    sequences = []
    labels = []

    # Generate reference for each species
    references = []
    for _ in range(n_species):
        ref = ''.join(rng.choice(bases, seq_length))
        references.append(ref)

    # Generate samples by mutating references
    for species_id, ref in enumerate(references):
        for _ in range(n_samples_per_species):
            seq = list(ref)
            # Apply mutations
            for i in range(len(seq)):
                if rng.random() < mutation_rate:
                    seq[i] = rng.choice(bases)
            sequences.append(''.join(seq))
            labels.append(species_id)

    # Shuffle
    indices = rng.permutation(len(sequences))
    sequences = [sequences[i] for i in indices]
    labels = [labels[i] for i in indices]

    return sequences, labels


def generate_splice_site_dataset(
    n_samples: int = 1000,
    window_size: int = 80,
    seed: int = 42
) -> Tuple[List[str], List[int]]:
    """
    Generate synthetic splice site dataset.

    Donor splice sites have GT at the splice point.
    Acceptor splice sites have AG at the splice point.
    Negatives are random.
    """
    rng = np.random.RandomState(seed)
    bases = ['A', 'C', 'G', 'T']

    sequences = []
    labels = []

    for i in range(n_samples):
        label = i % 3  # 0: negative, 1: donor (GT), 2: acceptor (AG)

        seq = list(''.join(rng.choice(bases, window_size)))
        center = window_size // 2

        if label == 1:
            # Donor site: GT + consensus
            seq[center] = 'G'
            seq[center + 1] = 'T'
            # Add consensus sequence around it
            if center >= 3:
                seq[center - 3:center] = list('MAG')  # M = A or C
                seq[center - 3] = rng.choice(['A', 'C'])
        elif label == 2:
            # Acceptor site: AG + polypyrimidine tract
            seq[center] = 'A'
            seq[center + 1] = 'G'
            # Polypyrimidine tract upstream
            for j in range(max(0, center - 15), center):
                seq[j] = rng.choice(['T', 'T', 'T', 'C'])  # T-rich

        sequences.append(''.join(seq))
        labels.append(label)

    # Shuffle
    indices = rng.permutation(len(sequences))
    sequences = [sequences[i] for i in indices]
    labels = [labels[i] for i in indices]

    return sequences, labels


# ============================================================================
# Training and Evaluation
# ============================================================================

def train_learned_hdc(
    model: LearnedHDCClassifier,
    train_seqs: List[str],
    train_labels: List[int],
    val_seqs: List[str],
    val_labels: List[int],
    epochs: int = 50,
    batch_size: int = 32,
    lr: float = 0.001,
    device: str = 'cpu'
) -> Dict[str, List[float]]:
    """Train the learned HDC model."""

    model = model.to(device)
    optimizer = torch.optim.Adam(model.parameters(), lr=lr)
    criterion = nn.CrossEntropyLoss()

    history = {
        'train_loss': [],
        'train_acc': [],
        'val_loss': [],
        'val_acc': [],
    }

    n_train = len(train_seqs)

    for epoch in range(epochs):
        model.train()

        # Shuffle training data
        indices = np.random.permutation(n_train)
        train_seqs_shuffled = [train_seqs[i] for i in indices]
        train_labels_shuffled = [train_labels[i] for i in indices]

        total_loss = 0
        correct = 0

        for i in range(0, n_train, batch_size):
            batch_seqs = train_seqs_shuffled[i:i+batch_size]
            batch_labels = torch.tensor(
                train_labels_shuffled[i:i+batch_size],
                dtype=torch.long,
                device=device
            )

            optimizer.zero_grad()
            logits = model(batch_seqs)
            loss = criterion(logits, batch_labels)
            loss.backward()
            optimizer.step()

            total_loss += loss.item() * len(batch_seqs)
            correct += (logits.argmax(dim=1) == batch_labels).sum().item()

        train_loss = total_loss / n_train
        train_acc = correct / n_train

        # Validation
        model.eval()
        with torch.no_grad():
            val_labels_tensor = torch.tensor(val_labels, dtype=torch.long, device=device)
            val_logits = model(val_seqs)
            val_loss = criterion(val_logits, val_labels_tensor).item()
            val_acc = (val_logits.argmax(dim=1) == val_labels_tensor).float().mean().item()

        history['train_loss'].append(train_loss)
        history['train_acc'].append(train_acc)
        history['val_loss'].append(val_loss)
        history['val_acc'].append(val_acc)

        if (epoch + 1) % 10 == 0:
            print(f"Epoch {epoch+1}/{epochs}: "
                  f"Train Loss={train_loss:.4f}, Train Acc={train_acc:.4f}, "
                  f"Val Loss={val_loss:.4f}, Val Acc={val_acc:.4f}")

    return history


def evaluate_baseline_hdc(
    encoder: BaselineHDCEncoder,
    train_seqs: List[str],
    train_labels: List[int],
    test_seqs: List[str],
    test_labels: List[int]
) -> float:
    """Evaluate baseline HDC using k-NN classification."""

    # Encode training data
    train_vecs = encoder.encode_batch(train_seqs)

    # Encode test data
    test_vecs = encoder.encode_batch(test_seqs)

    # k-NN classification (k=5)
    k = 5
    correct = 0

    for i, test_vec in enumerate(test_vecs):
        # Compute similarities to all training vectors
        similarities = np.array([
            encoder.similarity(test_vec, train_vec)
            for train_vec in train_vecs
        ])

        # Get top-k neighbors
        top_k_indices = np.argsort(similarities)[-k:]
        top_k_labels = [train_labels[j] for j in top_k_indices]

        # Majority vote
        from collections import Counter
        predicted = Counter(top_k_labels).most_common(1)[0][0]

        if predicted == test_labels[i]:
            correct += 1

    return correct / len(test_seqs)


def benchmark_speed(
    learned_model: LearnedHDCClassifier,
    baseline_encoder: BaselineHDCEncoder,
    sequences: List[str],
    n_runs: int = 5
) -> Dict[str, float]:
    """Benchmark encoding speed."""

    results = {}

    # Learned HDC
    learned_model.eval()
    times = []
    with torch.no_grad():
        for _ in range(n_runs):
            start = time.time()
            _ = learned_model.get_embeddings(sequences)
            times.append(time.time() - start)
    results['learned_hdc_ms'] = np.mean(times) * 1000
    results['learned_hdc_seqs_per_sec'] = len(sequences) / np.mean(times)

    # Baseline HDC
    times = []
    for _ in range(n_runs):
        start = time.time()
        _ = baseline_encoder.encode_batch(sequences)
        times.append(time.time() - start)
    results['baseline_hdc_ms'] = np.mean(times) * 1000
    results['baseline_hdc_seqs_per_sec'] = len(sequences) / np.mean(times)

    return results


# ============================================================================
# Main Experiment
# ============================================================================

def run_experiment(
    dataset_name: str = 'promoter',
    n_samples: int = 2000,
    test_split: float = 0.2,
    epochs: int = 50,
    kmer_length: int = 6,
    seed: int = 42
):
    """Run complete experiment comparing learned vs baseline HDC."""

    print(f"\n{'='*60}")
    print(f"LEARNED HDC EXPERIMENT: {dataset_name.upper()}")
    print(f"{'='*60}\n")

    # Generate dataset
    print("Generating dataset...")
    if dataset_name == 'promoter':
        sequences, labels = generate_promoter_dataset(n_samples, seed=seed)
        num_classes = 2
    elif dataset_name == 'taxonomy':
        sequences, labels = generate_taxonomy_dataset(
            n_species=10, n_samples_per_species=n_samples//10, seed=seed
        )
        num_classes = 10
    elif dataset_name == 'splice':
        sequences, labels = generate_splice_site_dataset(n_samples, seed=seed)
        num_classes = 3
    else:
        raise ValueError(f"Unknown dataset: {dataset_name}")

    # Split
    n_test = int(len(sequences) * test_split)
    train_seqs, test_seqs = sequences[n_test:], sequences[:n_test]
    train_labels, test_labels = labels[n_test:], labels[:n_test]

    print(f"  Train: {len(train_seqs)} sequences")
    print(f"  Test: {len(test_seqs)} sequences")
    print(f"  Classes: {num_classes}")
    print(f"  Sequence length: ~{len(sequences[0])} bp")

    # Configuration
    config = HDCConfig(
        dim=HYPERVECTOR_DIM,
        kmer_length=kmer_length,
        num_classes=num_classes,
        use_position_encoding=True,
    )

    # -------------------------------------------------------------------------
    # Baseline HDC (random vectors + k-NN)
    # -------------------------------------------------------------------------
    print("\n" + "-"*40)
    print("BASELINE HDC (Random Vectors + k-NN)")
    print("-"*40)

    baseline = BaselineHDCEncoder(config, seed=seed)

    start = time.time()
    baseline_acc = evaluate_baseline_hdc(
        baseline, train_seqs, train_labels, test_seqs, test_labels
    )
    baseline_time = time.time() - start

    print(f"  Accuracy: {baseline_acc*100:.2f}%")
    print(f"  Time: {baseline_time:.2f}s")

    # -------------------------------------------------------------------------
    # Learned HDC (no refinement head - pure HDC)
    # -------------------------------------------------------------------------
    print("\n" + "-"*40)
    print("LEARNED HDC (No Refinement Head)")
    print("-"*40)

    learned_pure = LearnedHDCClassifier(config, use_refinement_head=False)

    history_pure = train_learned_hdc(
        learned_pure, train_seqs, train_labels, test_seqs, test_labels,
        epochs=epochs, batch_size=32, lr=0.001
    )

    learned_pure_acc = history_pure['val_acc'][-1]
    print(f"  Final Accuracy: {learned_pure_acc*100:.2f}%")

    # -------------------------------------------------------------------------
    # Learned HDC (with refinement head)
    # -------------------------------------------------------------------------
    print("\n" + "-"*40)
    print("LEARNED HDC (With MLP Refinement)")
    print("-"*40)

    learned_mlp = LearnedHDCClassifier(config, use_refinement_head=True)

    history_mlp = train_learned_hdc(
        learned_mlp, train_seqs, train_labels, test_seqs, test_labels,
        epochs=epochs, batch_size=32, lr=0.001
    )

    learned_mlp_acc = history_mlp['val_acc'][-1]
    print(f"  Final Accuracy: {learned_mlp_acc*100:.2f}%")

    # -------------------------------------------------------------------------
    # Speed Benchmark
    # -------------------------------------------------------------------------
    print("\n" + "-"*40)
    print("SPEED BENCHMARK")
    print("-"*40)

    speed_results = benchmark_speed(learned_mlp, baseline, test_seqs[:100])

    print(f"  Baseline HDC: {speed_results['baseline_hdc_seqs_per_sec']:.0f} seq/s")
    print(f"  Learned HDC: {speed_results['learned_hdc_seqs_per_sec']:.0f} seq/s")

    # -------------------------------------------------------------------------
    # Summary
    # -------------------------------------------------------------------------
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    print(f"\n  Dataset: {dataset_name}")
    print(f"  K-mer length: {kmer_length}")
    print(f"  HDC dimension: {HYPERVECTOR_DIM:,}")
    print(f"\n  {'Method':<30} {'Accuracy':>10}")
    print(f"  {'-'*40}")
    print(f"  {'Baseline HDC (random)':<30} {baseline_acc*100:>9.2f}%")
    print(f"  {'Learned HDC (pure)':<30} {learned_pure_acc*100:>9.2f}%")
    print(f"  {'Learned HDC + MLP':<30} {learned_mlp_acc*100:>9.2f}%")

    improvement = (learned_mlp_acc - baseline_acc) / baseline_acc * 100
    print(f"\n  Improvement over baseline: {improvement:+.1f}%")

    return {
        'baseline_acc': baseline_acc,
        'learned_pure_acc': learned_pure_acc,
        'learned_mlp_acc': learned_mlp_acc,
        'speed': speed_results,
        'history_pure': history_pure,
        'history_mlp': history_mlp,
    }


if __name__ == '__main__':
    # Run experiments on all datasets
    results = {}

    for dataset in ['promoter', 'taxonomy', 'splice']:
        results[dataset] = run_experiment(
            dataset_name=dataset,
            n_samples=2000,
            epochs=50,
            kmer_length=6,
        )

    # Final comparison
    print("\n" + "="*70)
    print("FINAL COMPARISON ACROSS ALL DATASETS")
    print("="*70)
    print(f"\n  {'Dataset':<15} {'Baseline':>12} {'Learned Pure':>14} {'Learned+MLP':>14}")
    print(f"  {'-'*55}")

    for dataset, r in results.items():
        print(f"  {dataset:<15} {r['baseline_acc']*100:>11.2f}% "
              f"{r['learned_pure_acc']*100:>13.2f}% "
              f"{r['learned_mlp_acc']*100:>13.2f}%")
