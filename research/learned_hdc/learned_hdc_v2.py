#!/usr/bin/env python3
"""
Learned HDC v2 - Improved Implementation

Key improvements over v1:
1. Adam optimizer (momentum + adaptive LR)
2. L2 regularization
3. Better initialization (Xavier-like)
4. MLP classification head option
5. Batch normalization equivalent
6. Learning rate scheduling
"""

import numpy as np
from typing import List, Tuple, Dict, Optional
from dataclasses import dataclass
import time
from collections import Counter

HYPERVECTOR_DIM = 2_000  # Reduced for speed


@dataclass
class HDCConfig:
    dim: int = HYPERVECTOR_DIM
    kmer_length: int = 6
    num_classes: int = 2
    learning_rate: float = 0.001  # Lower for Adam
    l2_reg: float = 0.01  # L2 regularization
    use_mlp: bool = True  # Use MLP head
    mlp_hidden: int = 256  # Hidden layer size


def generate_kmers(k: int) -> List[str]:
    """Generate all possible k-mers."""
    bases = ['A', 'C', 'G', 'T']
    if k == 1:
        return bases
    smaller = generate_kmers(k - 1)
    return [b + s for b in bases for s in smaller]


def softmax(x):
    """Stable softmax."""
    exp_x = np.exp(x - np.max(x, axis=-1, keepdims=True))
    return exp_x / (exp_x.sum(axis=-1, keepdims=True) + 1e-10)


def relu(x):
    return np.maximum(0, x)


def relu_grad(x):
    return (x > 0).astype(np.float32)


class AdamOptimizer:
    """Adam optimizer for individual parameters."""

    def __init__(self, shape, lr=0.001, beta1=0.9, beta2=0.999, eps=1e-8):
        self.lr = lr
        self.beta1 = beta1
        self.beta2 = beta2
        self.eps = eps
        self.m = np.zeros(shape)
        self.v = np.zeros(shape)
        self.t = 0

    def update(self, param, grad):
        self.t += 1
        self.m = self.beta1 * self.m + (1 - self.beta1) * grad
        self.v = self.beta2 * self.v + (1 - self.beta2) * (grad ** 2)

        m_hat = self.m / (1 - self.beta1 ** self.t)
        v_hat = self.v / (1 - self.beta2 ** self.t)

        param -= self.lr * m_hat / (np.sqrt(v_hat) + self.eps)
        return param


class LearnedHDCv2:
    """
    Improved Learned HDC with:
    - Adam optimizer
    - L2 regularization
    - MLP classification head
    - Better initialization
    """

    def __init__(self, config: HDCConfig, seed: int = 42):
        self.config = config
        self.rng = np.random.RandomState(seed)

        num_kmers = 4 ** config.kmer_length

        # Xavier-like initialization for embeddings
        scale = np.sqrt(2.0 / config.dim)
        self.embeddings = self.rng.randn(num_kmers, config.dim) * scale

        # K-mer mapping
        self.kmers = generate_kmers(config.kmer_length)
        self.kmer_to_idx = {kmer: i for i, kmer in enumerate(self.kmers)}

        # Classification head
        if config.use_mlp:
            # MLP: dim -> hidden -> classes
            self.W1 = self.rng.randn(config.dim, config.mlp_hidden) * np.sqrt(2.0 / config.dim)
            self.b1 = np.zeros(config.mlp_hidden)
            self.W2 = self.rng.randn(config.mlp_hidden, config.num_classes) * np.sqrt(2.0 / config.mlp_hidden)
            self.b2 = np.zeros(config.num_classes)

            # Adam optimizers
            self.opt_W1 = AdamOptimizer(self.W1.shape, lr=config.learning_rate)
            self.opt_b1 = AdamOptimizer(self.b1.shape, lr=config.learning_rate)
            self.opt_W2 = AdamOptimizer(self.W2.shape, lr=config.learning_rate)
            self.opt_b2 = AdamOptimizer(self.b2.shape, lr=config.learning_rate)
        else:
            # Linear: dim -> classes
            self.W = self.rng.randn(config.dim, config.num_classes) * np.sqrt(2.0 / config.dim)
            self.b = np.zeros(config.num_classes)
            self.opt_W = AdamOptimizer(self.W.shape, lr=config.learning_rate)
            self.opt_b = AdamOptimizer(self.b.shape, lr=config.learning_rate)

        # Embedding optimizer (sparse updates, simpler)
        self.embed_m = np.zeros_like(self.embeddings)
        self.embed_v = np.zeros_like(self.embeddings)
        self.embed_t = 0

    def encode(self, sequence: str) -> np.ndarray:
        """Encode sequence using learned embeddings."""
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

        # Mean pooling with normalization
        vectors = self.embeddings[indices]
        encoding = vectors.mean(axis=0)

        # L2 normalize for stability
        norm = np.linalg.norm(encoding)
        if norm > 0:
            encoding = encoding / norm

        return encoding

    def encode_binary(self, sequence: str) -> np.ndarray:
        """Binary encoding for inference."""
        return (self.encode(sequence) > 0).astype(np.float32)

    def forward(self, encodings: np.ndarray) -> Tuple[np.ndarray, Dict]:
        """Forward pass through classification head."""
        if self.config.use_mlp:
            h1 = encodings @ self.W1 + self.b1  # (batch, hidden)
            a1 = relu(h1)  # Activation
            logits = a1 @ self.W2 + self.b2  # (batch, classes)
            return logits, {'h1': h1, 'a1': a1, 'encodings': encodings}
        else:
            logits = encodings @ self.W + self.b
            return logits, {'encodings': encodings}

    def predict(self, sequences: List[str]) -> np.ndarray:
        """Predict class labels."""
        encodings = np.stack([self.encode(seq) for seq in sequences])
        logits, _ = self.forward(encodings)
        return np.argmax(logits, axis=1)

    def train_step(self, sequences: List[str], labels: List[int]) -> float:
        """Training step with Adam optimizer."""
        batch_size = len(sequences)
        k = self.config.kmer_length

        # Forward
        encodings = []
        kmer_indices_batch = []

        for seq in sequences:
            indices = []
            for i in range(len(seq) - k + 1):
                kmer = seq[i:i+k].upper()
                idx = self.kmer_to_idx.get(kmer)
                if idx is not None:
                    indices.append(idx)

            if indices:
                vecs = self.embeddings[indices]
                enc = vecs.mean(axis=0)
                norm = np.linalg.norm(enc)
                if norm > 0:
                    enc = enc / norm
            else:
                enc = np.zeros(self.config.dim)

            encodings.append(enc)
            kmer_indices_batch.append(indices)

        encodings = np.stack(encodings)
        logits, cache = self.forward(encodings)

        # Softmax + cross-entropy
        probs = softmax(logits)

        # One-hot labels
        labels_onehot = np.zeros((batch_size, self.config.num_classes))
        for i, l in enumerate(labels):
            labels_onehot[i, l] = 1

        # Loss with L2 regularization
        ce_loss = -np.mean(np.sum(labels_onehot * np.log(probs + 1e-10), axis=1))

        l2_loss = 0
        if self.config.use_mlp:
            l2_loss = 0.5 * self.config.l2_reg * (
                np.sum(self.W1 ** 2) + np.sum(self.W2 ** 2)
            )
        else:
            l2_loss = 0.5 * self.config.l2_reg * np.sum(self.W ** 2)

        loss = ce_loss + l2_loss

        # Backward
        d_logits = (probs - labels_onehot) / batch_size

        if self.config.use_mlp:
            # MLP backward
            d_W2 = cache['a1'].T @ d_logits + self.config.l2_reg * self.W2
            d_b2 = d_logits.sum(axis=0)

            d_a1 = d_logits @ self.W2.T
            d_h1 = d_a1 * relu_grad(cache['h1'])

            d_W1 = encodings.T @ d_h1 + self.config.l2_reg * self.W1
            d_b1 = d_h1.sum(axis=0)

            d_encodings = d_h1 @ self.W1.T

            # Update with Adam
            self.W2 = self.opt_W2.update(self.W2, d_W2)
            self.b2 = self.opt_b2.update(self.b2, d_b2)
            self.W1 = self.opt_W1.update(self.W1, d_W1)
            self.b1 = self.opt_b1.update(self.b1, d_b1)
        else:
            d_W = encodings.T @ d_logits + self.config.l2_reg * self.W
            d_b = d_logits.sum(axis=0)
            d_encodings = d_logits @ self.W.T

            self.W = self.opt_W.update(self.W, d_W)
            self.b = self.opt_b.update(self.b, d_b)

        # Update embeddings (Adam with sparse updates)
        lr = self.config.learning_rate
        self.embed_t += 1
        beta1, beta2, eps = 0.9, 0.999, 1e-8

        for i, (d_enc, indices) in enumerate(zip(d_encodings, kmer_indices_batch)):
            if indices:
                grad = d_enc / len(indices)
                for idx in indices:
                    self.embed_m[idx] = beta1 * self.embed_m[idx] + (1 - beta1) * grad
                    self.embed_v[idx] = beta2 * self.embed_v[idx] + (1 - beta2) * (grad ** 2)

                    m_hat = self.embed_m[idx] / (1 - beta1 ** self.embed_t)
                    v_hat = self.embed_v[idx] / (1 - beta2 ** self.embed_t)

                    self.embeddings[idx] -= lr * m_hat / (np.sqrt(v_hat) + eps)

        return loss


class BaselineHDCEncoder:
    """Baseline HDC with random k-mer vectors (for comparison)."""

    def __init__(self, config: HDCConfig, seed: int = 42):
        self.config = config
        self.rng = np.random.RandomState(seed)

        num_kmers = 4 ** config.kmer_length
        self.embeddings = self.rng.randint(0, 2, (num_kmers, config.dim)).astype(np.float32)

        self.kmers = generate_kmers(config.kmer_length)
        self.kmer_to_idx = {kmer: i for i, kmer in enumerate(self.kmers)}

    def encode(self, sequence: str) -> np.ndarray:
        k = self.config.kmer_length
        if len(sequence) < k:
            return np.zeros(self.config.dim)

        acc = np.zeros(self.config.dim)
        count = 0

        for i in range(len(sequence) - k + 1):
            kmer = sequence[i:i+k].upper()
            idx = self.kmer_to_idx.get(kmer)
            if idx is not None:
                acc += self.embeddings[idx]
                count += 1

        if count == 0:
            return np.zeros(self.config.dim)

        return (acc > count / 2).astype(np.float32)

    def encode_batch(self, sequences: List[str]) -> np.ndarray:
        return np.stack([self.encode(seq) for seq in sequences])


def generate_promoter_dataset(n_samples: int, seq_length: int = 100, seed: int = 42):
    """Generate synthetic promoter dataset with stronger signal."""
    rng = np.random.RandomState(seed)
    bases = ['A', 'C', 'G', 'T']
    sequences, labels = [], []

    # Stronger motifs for promoters
    tata_box = "TATAAA"
    caat_box = "CCAAT"
    gc_box = "GGGCGG"

    for i in range(n_samples):
        seq = list(''.join(rng.choice(bases, seq_length)))

        if i < n_samples // 2:
            # Promoter: insert multiple motifs
            # TATA box at position 25-30
            pos = rng.randint(20, 30)
            for j, c in enumerate(tata_box):
                if pos + j < seq_length:
                    seq[pos + j] = c

            # Sometimes add CAAT box
            if rng.random() < 0.5:
                caat_pos = rng.randint(45, 55)
                for j, c in enumerate(caat_box):
                    if caat_pos + j < seq_length:
                        seq[caat_pos + j] = c

            labels.append(1)
        else:
            labels.append(0)

        sequences.append(''.join(seq))

    idx = rng.permutation(len(sequences))
    return [sequences[i] for i in idx], [labels[i] for i in idx]


def generate_taxonomy_dataset(n_species: int, n_per_species: int, seq_length: int = 200,
                              mutation_rate: float = 0.1, seed: int = 42):
    """Generate taxonomy dataset with higher mutation rate."""
    rng = np.random.RandomState(seed)
    bases = ['A', 'C', 'G', 'T']
    sequences, labels = [], []

    references = [''.join(rng.choice(bases, seq_length)) for _ in range(n_species)]

    for species_id, ref in enumerate(references):
        for _ in range(n_per_species):
            seq = list(ref)
            for i in range(len(seq)):
                if rng.random() < mutation_rate:
                    seq[i] = rng.choice(bases)
            sequences.append(''.join(seq))
            labels.append(species_id)

    idx = rng.permutation(len(sequences))
    return [sequences[i] for i in idx], [labels[i] for i in idx]


def evaluate_knn(encoder, train_seqs, train_labels, test_seqs, test_labels, k=5):
    """Evaluate using k-NN."""
    if hasattr(encoder, 'encode_batch'):
        train_vecs = encoder.encode_batch(train_seqs)
    else:
        train_vecs = np.stack([encoder.encode_binary(s) for s in train_seqs])

    if hasattr(encoder, 'encode_batch'):
        test_vecs = encoder.encode_batch(test_seqs)
    else:
        test_vecs = np.stack([encoder.encode_binary(s) for s in test_seqs])

    correct = 0
    for i, tv in enumerate(test_vecs):
        sims = np.mean(train_vecs == tv, axis=1)
        top_k = np.argsort(sims)[-k:]
        votes = [train_labels[j] for j in top_k]
        pred = Counter(votes).most_common(1)[0][0]
        if pred == test_labels[i]:
            correct += 1

    return correct / len(test_seqs)


def run_experiment(dataset: str = 'promoter', n_samples: int = 1000, epochs: int = 50):
    """Run experiment comparing v1 and v2."""
    print(f"\n{'='*70}")
    print(f"LEARNED HDC v2 EXPERIMENT: {dataset.upper()}")
    print(f"{'='*70}\n", flush=True)

    # Generate data
    print("Generating dataset...", flush=True)
    if dataset == 'promoter':
        sequences, labels = generate_promoter_dataset(n_samples)
        num_classes = 2
    else:
        sequences, labels = generate_taxonomy_dataset(10, n_samples // 10, mutation_rate=0.15)
        num_classes = 10

    split = int(0.8 * len(sequences))
    train_seqs, test_seqs = sequences[:split], sequences[split:]
    train_labels, test_labels = labels[:split], labels[split:]

    print(f"  Train: {len(train_seqs)}, Test: {len(test_seqs)}")
    print(f"  Classes: {num_classes}", flush=True)

    # =========================================================================
    # Baseline HDC
    # =========================================================================
    print("\n" + "-"*50)
    print("BASELINE HDC (Random Vectors + k-NN)")
    print("-"*50, flush=True)

    config_base = HDCConfig(dim=HYPERVECTOR_DIM, kmer_length=6, num_classes=num_classes)
    baseline = BaselineHDCEncoder(config_base)

    t0 = time.time()
    baseline_acc = evaluate_knn(baseline, train_seqs, train_labels, test_seqs, test_labels)
    baseline_time = time.time() - t0
    print(f"  Accuracy: {baseline_acc*100:.2f}%")
    print(f"  Time: {baseline_time:.2f}s", flush=True)

    # =========================================================================
    # Learned HDC v2 (MLP head)
    # =========================================================================
    print("\n" + "-"*50)
    print("LEARNED HDC v2 (Adam + MLP + L2 Regularization)")
    print("-"*50, flush=True)

    config_v2 = HDCConfig(
        dim=HYPERVECTOR_DIM,
        kmer_length=6,
        num_classes=num_classes,
        learning_rate=0.001,
        l2_reg=0.01,
        use_mlp=True,
        mlp_hidden=256
    )

    learned_v2 = LearnedHDCv2(config_v2)
    batch_size = 32

    t0 = time.time()
    for epoch in range(epochs):
        idx = np.random.permutation(len(train_seqs))
        train_seqs_shuf = [train_seqs[i] for i in idx]
        train_labels_shuf = [train_labels[i] for i in idx]

        total_loss = 0
        for i in range(0, len(train_seqs), batch_size):
            batch_seqs = train_seqs_shuf[i:i+batch_size]
            batch_labels = train_labels_shuf[i:i+batch_size]
            loss = learned_v2.train_step(batch_seqs, batch_labels)
            total_loss += loss * len(batch_seqs)

        if (epoch + 1) % 10 == 0:
            preds = learned_v2.predict(test_seqs)
            acc = np.mean(np.array(preds) == np.array(test_labels))
            print(f"  Epoch {epoch+1}: Loss={total_loss/len(train_seqs):.4f}, "
                  f"Test Acc={acc*100:.2f}%", flush=True)

    learned_v2_time = time.time() - t0

    preds = learned_v2.predict(test_seqs)
    learned_v2_acc = np.mean(np.array(preds) == np.array(test_labels))
    print(f"  Final Accuracy: {learned_v2_acc*100:.2f}%")
    print(f"  Training Time: {learned_v2_time:.2f}s", flush=True)

    # k-NN with learned embeddings
    learned_v2_knn_acc = evaluate_knn(learned_v2, train_seqs, train_labels, test_seqs, test_labels)
    print(f"  Learned k-NN Accuracy: {learned_v2_knn_acc*100:.2f}%", flush=True)

    # =========================================================================
    # Summary
    # =========================================================================
    print("\n" + "="*70)
    print("SUMMARY")
    print("="*70)
    print(f"\n  {'Method':<40} {'Accuracy':>12}")
    print(f"  {'-'*52}")
    print(f"  {'Baseline HDC (random + k-NN)':<40} {baseline_acc*100:>11.2f}%")
    print(f"  {'Learned HDC v2 (classifier)':<40} {learned_v2_acc*100:>11.2f}%")
    print(f"  {'Learned HDC v2 (k-NN)':<40} {learned_v2_knn_acc*100:>11.2f}%")

    improvement = (learned_v2_acc - baseline_acc) / baseline_acc * 100 if baseline_acc > 0 else 0
    print(f"\n  Improvement (classifier vs baseline): {improvement:+.1f}%", flush=True)

    return {
        'baseline_acc': baseline_acc,
        'learned_v2_acc': learned_v2_acc,
        'learned_v2_knn_acc': learned_v2_knn_acc,
        'improvement': improvement
    }


if __name__ == '__main__':
    print("\n" + "="*70)
    print("  LEARNED HDC v2 - Improved Implementation")
    print("  Features: Adam optimizer, MLP head, L2 regularization")
    print("="*70, flush=True)

    results = {}

    for dataset in ['promoter', 'taxonomy']:
        results[dataset] = run_experiment(dataset, n_samples=1000, epochs=50)

    # Final summary
    print("\n" + "="*70)
    print("FINAL RESULTS")
    print("="*70)
    print(f"\n  {'Dataset':<15} {'Baseline':>12} {'Learned v2':>12} {'Improvement':>14}")
    print(f"  {'-'*55}")

    for ds, r in results.items():
        print(f"  {ds:<15} {r['baseline_acc']*100:>11.2f}% "
              f"{r['learned_v2_acc']*100:>11.2f}% "
              f"{r['improvement']:>+13.1f}%")

    print("\n" + "="*70)
    print("KEY IMPROVEMENTS IN v2:")
    print("="*70)
    print("""
  1. Adam optimizer - adaptive learning rates for each parameter
  2. MLP classification head - captures non-linear patterns
  3. L2 regularization - prevents overfitting
  4. Better initialization - Xavier-like scaling
  5. Normalized encodings - L2 normalization for stability

  If v2 still underperforms, potential issues:
  - Need more training data
  - K-mer length might need tuning
  - Consider multi-scale k-mers
  - May need contrastive pre-training
""", flush=True)
