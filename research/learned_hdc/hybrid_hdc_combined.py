#!/usr/bin/env python3
"""
Hybrid HDC Combined: Contrastive Pre-training + Learned Fine-tuning

This combines the best of both approaches:
1. Stage 1: Contrastive pre-training on unlabeled sequences
2. Stage 2: Supervised fine-tuning with learned embeddings + classifier

Expected: ~95%+ accuracy on promoter detection
"""

import numpy as np
from typing import List, Tuple, Dict, Optional
from dataclasses import dataclass
import time
from collections import Counter
import os

HYPERVECTOR_DIM = 1_000


@dataclass
class HybridConfig:
    dim: int = HYPERVECTOR_DIM
    kmer_length: int = 6
    num_classes: int = 2
    # Contrastive pre-training
    contrastive_lr: float = 0.01
    contrastive_epochs: int = 20
    temperature: float = 0.1
    num_negatives: int = 8
    # Fine-tuning
    finetune_lr: float = 0.001
    finetune_epochs: int = 50
    l2_reg: float = 0.001
    mlp_hidden: int = 256
    # Early stopping
    patience: int = 5
    min_delta: float = 0.001


def generate_kmers(k: int) -> List[str]:
    bases = ['A', 'C', 'G', 'T']
    if k == 1:
        return bases
    smaller = generate_kmers(k - 1)
    return [b + s for b in bases for s in smaller]


def cosine_similarity(a: np.ndarray, b: np.ndarray) -> float:
    norm_a = np.linalg.norm(a)
    norm_b = np.linalg.norm(b)
    if norm_a == 0 or norm_b == 0:
        return 0
    return np.dot(a, b) / (norm_a * norm_b)


def softmax(x, axis=-1):
    exp_x = np.exp(x - np.max(x, axis=axis, keepdims=True))
    return exp_x / (exp_x.sum(axis=axis, keepdims=True) + 1e-10)


def relu(x):
    return np.maximum(0, x)


def augment_sequence(sequence: str, rng: np.random.RandomState,
                    mutation_rate: float = 0.1) -> str:
    """DNA sequence augmentation."""
    bases = ['A', 'C', 'G', 'T']
    seq = list(sequence)
    for i in range(len(seq)):
        if rng.random() < mutation_rate:
            seq[i] = rng.choice(bases)
    return ''.join(seq)


class AdamOptimizer:
    """Adam optimizer for a single parameter."""
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


class HybridHDCModel:
    """
    Combined Contrastive + Learned HDC Model.

    Training pipeline:
    1. Initialize k-mer embeddings randomly
    2. Pre-train with contrastive learning (no labels)
    3. Fine-tune embeddings + MLP classifier (with labels)
    """

    def __init__(self, config: HybridConfig, seed: int = 42):
        self.config = config
        self.rng = np.random.RandomState(seed)

        num_kmers = 4 ** config.kmer_length

        # K-mer embeddings (will be pre-trained then fine-tuned)
        scale = np.sqrt(2.0 / config.dim)
        self.embeddings = self.rng.randn(num_kmers, config.dim) * scale

        # K-mer mapping
        self.kmers = generate_kmers(config.kmer_length)
        self.kmer_to_idx = {kmer: i for i, kmer in enumerate(self.kmers)}

        # MLP classifier (initialized later during fine-tuning)
        self.W1 = None
        self.b1 = None
        self.W2 = None
        self.b2 = None

        # Adam state for embeddings
        self.embed_m = np.zeros_like(self.embeddings)
        self.embed_v = np.zeros_like(self.embeddings)
        self.embed_t = 0

        # Training state
        self.is_pretrained = False
        self.is_finetuned = False

    def encode(self, sequence: str, normalize: bool = True) -> np.ndarray:
        """Encode sequence using current embeddings."""
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

        vectors = self.embeddings[indices]
        encoding = vectors.mean(axis=0)

        if normalize:
            norm = np.linalg.norm(encoding)
            if norm > 0:
                encoding = encoding / norm

        return encoding

    def encode_batch(self, sequences: List[str], normalize: bool = True) -> np.ndarray:
        return np.stack([self.encode(seq, normalize) for seq in sequences])

    # =========================================================================
    # STAGE 1: CONTRASTIVE PRE-TRAINING
    # =========================================================================

    def contrastive_pretrain(self, sequences: List[str],
                             verbose: bool = True) -> List[float]:
        """
        Pre-train embeddings using contrastive learning.

        For each sequence:
        - Create augmented version (positive)
        - Sample other sequences as negatives
        - Push positive close, negatives far
        """
        config = self.config
        losses = []

        if verbose:
            print("\n" + "="*60)
            print("STAGE 1: CONTRASTIVE PRE-TRAINING")
            print(f"  Sequences: {len(sequences)}")
            print(f"  Temperature: {config.temperature}")
            print(f"  Epochs: {config.contrastive_epochs}")
            print("="*60, flush=True)

        batch_size = 32

        for epoch in range(config.contrastive_epochs):
            idx = self.rng.permutation(len(sequences))
            seqs_shuf = [sequences[i] for i in idx]

            epoch_loss = 0
            n_batches = 0

            for i in range(0, len(seqs_shuf), batch_size):
                batch = seqs_shuf[i:i+batch_size]
                if len(batch) < 2:
                    continue

                loss = self._contrastive_step(batch)
                epoch_loss += loss
                n_batches += 1

            avg_loss = epoch_loss / max(n_batches, 1)
            losses.append(avg_loss)

            if verbose and (epoch + 1) % 5 == 0:
                print(f"  Epoch {epoch+1}: Contrastive Loss = {avg_loss:.4f}", flush=True)

        self.is_pretrained = True
        if verbose:
            print(f"  Pre-training complete!", flush=True)

        return losses

    def _contrastive_step(self, sequences: List[str]) -> float:
        """Single contrastive training step."""
        config = self.config
        batch_size = len(sequences)

        # Create augmented positives
        augmented = [augment_sequence(seq, self.rng) for seq in sequences]

        # Encode all
        anchors = [self.encode(seq) for seq in sequences]
        positives = [self.encode(aug) for aug in augmented]

        # Get k-mer indices for gradient updates
        k = config.kmer_length
        anchor_indices = []
        for seq in sequences:
            indices = []
            for i in range(len(seq) - k + 1):
                kmer = seq[i:i+k].upper()
                idx = self.kmer_to_idx.get(kmer)
                if idx is not None:
                    indices.append(idx)
            anchor_indices.append(indices)

        total_loss = 0
        self.embed_t += 1
        beta1, beta2, eps = 0.9, 0.999, 1e-8
        lr = config.contrastive_lr

        for i in range(batch_size):
            # Sample negatives
            neg_idx = [j for j in range(batch_size) if j != i]
            self.rng.shuffle(neg_idx)
            neg_idx = neg_idx[:config.num_negatives]
            negatives = np.array([anchors[j] for j in neg_idx])

            # InfoNCE loss
            sim_pos = cosine_similarity(anchors[i], positives[i]) / config.temperature
            sim_negs = np.array([cosine_similarity(anchors[i], neg) / config.temperature
                                for neg in negatives])

            exp_pos = np.exp(np.clip(sim_pos, -20, 20))
            exp_negs = np.exp(np.clip(sim_negs, -20, 20))
            denom = exp_pos + exp_negs.sum()

            loss = -np.log(exp_pos / (denom + 1e-10))
            total_loss += loss

            # Gradient (simplified)
            probs = np.concatenate([[exp_pos], exp_negs]) / denom
            weighted_sum = probs[0] * positives[i]
            for j, neg in enumerate(negatives):
                weighted_sum += probs[j+1] * neg

            d_anchor = (weighted_sum - positives[i]) / config.temperature

            # Update embeddings
            if anchor_indices[i]:
                grad = d_anchor / len(anchor_indices[i])
                for idx in anchor_indices[i]:
                    self.embed_m[idx] = beta1 * self.embed_m[idx] + (1 - beta1) * grad
                    self.embed_v[idx] = beta2 * self.embed_v[idx] + (1 - beta2) * (grad ** 2)

                    m_hat = self.embed_m[idx] / (1 - beta1 ** self.embed_t)
                    v_hat = self.embed_v[idx] / (1 - beta2 ** self.embed_t)

                    self.embeddings[idx] -= lr * m_hat / (np.sqrt(v_hat) + eps)

        return total_loss / batch_size

    # =========================================================================
    # STAGE 2: SUPERVISED FINE-TUNING
    # =========================================================================

    def finetune(self, train_seqs: List[str], train_labels: List[int],
                 val_seqs: Optional[List[str]] = None,
                 val_labels: Optional[List[int]] = None,
                 verbose: bool = True) -> Dict:
        """
        Fine-tune embeddings + train MLP classifier.

        Uses early stopping based on validation accuracy.
        """
        config = self.config

        # Initialize MLP classifier
        self.W1 = self.rng.randn(config.dim, config.mlp_hidden) * np.sqrt(2.0 / config.dim)
        self.b1 = np.zeros(config.mlp_hidden)
        self.W2 = self.rng.randn(config.mlp_hidden, config.num_classes) * np.sqrt(2.0 / config.mlp_hidden)
        self.b2 = np.zeros(config.num_classes)

        # Optimizers
        self.opt_W1 = AdamOptimizer(self.W1.shape, lr=config.finetune_lr)
        self.opt_b1 = AdamOptimizer(self.b1.shape, lr=config.finetune_lr)
        self.opt_W2 = AdamOptimizer(self.W2.shape, lr=config.finetune_lr)
        self.opt_b2 = AdamOptimizer(self.b2.shape, lr=config.finetune_lr)

        if verbose:
            print("\n" + "="*60)
            print("STAGE 2: SUPERVISED FINE-TUNING")
            print(f"  Train samples: {len(train_seqs)}")
            print(f"  Validation: {len(val_seqs) if val_seqs else 0}")
            print(f"  Epochs: {config.finetune_epochs}")
            print(f"  Early stopping patience: {config.patience}")
            print("="*60, flush=True)

        batch_size = 32
        best_val_acc = 0
        best_epoch = 0
        patience_counter = 0
        history = {'train_loss': [], 'val_acc': []}

        for epoch in range(config.finetune_epochs):
            # Shuffle training data
            idx = self.rng.permutation(len(train_seqs))
            train_seqs_shuf = [train_seqs[i] for i in idx]
            train_labels_shuf = [train_labels[i] for i in idx]

            epoch_loss = 0
            for i in range(0, len(train_seqs), batch_size):
                batch_seqs = train_seqs_shuf[i:i+batch_size]
                batch_labels = train_labels_shuf[i:i+batch_size]
                loss = self._finetune_step(batch_seqs, batch_labels)
                epoch_loss += loss * len(batch_labels)

            avg_loss = epoch_loss / len(train_seqs)
            history['train_loss'].append(avg_loss)

            # Validation
            if val_seqs:
                val_preds = self.predict(val_seqs)
                val_acc = np.mean(np.array(val_preds) == np.array(val_labels))
                history['val_acc'].append(val_acc)

                # Early stopping
                if val_acc > best_val_acc + config.min_delta:
                    best_val_acc = val_acc
                    best_epoch = epoch + 1
                    patience_counter = 0
                    # Save best weights (simplified - just track)
                else:
                    patience_counter += 1

                if verbose and (epoch + 1) % 5 == 0:
                    print(f"  Epoch {epoch+1}: Loss={avg_loss:.4f}, "
                          f"Val Acc={val_acc*100:.2f}%", flush=True)

                if patience_counter >= config.patience:
                    if verbose:
                        print(f"  Early stopping at epoch {epoch+1}", flush=True)
                    break
            else:
                if verbose and (epoch + 1) % 10 == 0:
                    print(f"  Epoch {epoch+1}: Loss={avg_loss:.4f}", flush=True)

        self.is_finetuned = True

        if verbose:
            print(f"\n  Best validation accuracy: {best_val_acc*100:.2f}% (epoch {best_epoch})",
                  flush=True)

        return {
            'best_val_acc': best_val_acc,
            'best_epoch': best_epoch,
            'history': history
        }

    def _finetune_step(self, sequences: List[str], labels: List[int]) -> float:
        """Single fine-tuning step."""
        config = self.config
        batch_size = len(sequences)
        k = config.kmer_length

        # Forward pass
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
                enc = np.zeros(config.dim)

            encodings.append(enc)
            kmer_indices_batch.append(indices)

        encodings = np.stack(encodings)

        # MLP forward
        h1 = encodings @ self.W1 + self.b1
        a1 = relu(h1)
        logits = a1 @ self.W2 + self.b2

        # Softmax + cross-entropy
        probs = softmax(logits, axis=-1)
        labels_onehot = np.zeros((batch_size, config.num_classes))
        for i, l in enumerate(labels):
            labels_onehot[i, l] = 1

        ce_loss = -np.mean(np.sum(labels_onehot * np.log(probs + 1e-10), axis=1))
        l2_loss = 0.5 * config.l2_reg * (np.sum(self.W1**2) + np.sum(self.W2**2))
        loss = ce_loss + l2_loss

        # Backward
        d_logits = (probs - labels_onehot) / batch_size

        d_W2 = a1.T @ d_logits + config.l2_reg * self.W2
        d_b2 = d_logits.sum(axis=0)

        d_a1 = d_logits @ self.W2.T
        d_h1 = d_a1 * (h1 > 0)

        d_W1 = encodings.T @ d_h1 + config.l2_reg * self.W1
        d_b1 = d_h1.sum(axis=0)

        d_encodings = d_h1 @ self.W1.T

        # Update MLP weights
        self.W2 = self.opt_W2.update(self.W2, d_W2)
        self.b2 = self.opt_b2.update(self.b2, d_b2)
        self.W1 = self.opt_W1.update(self.W1, d_W1)
        self.b1 = self.opt_b1.update(self.b1, d_b1)

        # Update embeddings
        lr = config.finetune_lr * 0.1  # Lower LR for embeddings
        beta1, beta2, eps = 0.9, 0.999, 1e-8
        self.embed_t += 1

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

    def predict(self, sequences: List[str]) -> np.ndarray:
        """Predict class labels."""
        if self.W1 is None:
            raise RuntimeError("Model must be fine-tuned before prediction")

        encodings = self.encode_batch(sequences)
        h1 = relu(encodings @ self.W1 + self.b1)
        logits = h1 @ self.W2 + self.b2
        return np.argmax(logits, axis=1)

    def predict_proba(self, sequences: List[str]) -> np.ndarray:
        """Predict class probabilities."""
        if self.W1 is None:
            raise RuntimeError("Model must be fine-tuned before prediction")

        encodings = self.encode_batch(sequences)
        h1 = relu(encodings @ self.W1 + self.b1)
        logits = h1 @ self.W2 + self.b2
        return softmax(logits, axis=-1)


# =============================================================================
# BASELINE FOR COMPARISON
# =============================================================================

class BaselineHDC:
    """Baseline random HDC + k-NN."""

    def __init__(self, dim: int, kmer_length: int, seed: int = 42):
        self.dim = dim
        self.kmer_length = kmer_length
        rng = np.random.RandomState(seed)

        num_kmers = 4 ** kmer_length
        self.embeddings = rng.randint(0, 2, (num_kmers, dim)).astype(np.float32)

        self.kmers = generate_kmers(kmer_length)
        self.kmer_to_idx = {kmer: i for i, kmer in enumerate(self.kmers)}

    def encode(self, sequence: str) -> np.ndarray:
        k = self.kmer_length
        if len(sequence) < k:
            return np.zeros(self.dim)

        acc = np.zeros(self.dim)
        count = 0
        for i in range(len(sequence) - k + 1):
            kmer = sequence[i:i+k].upper()
            idx = self.kmer_to_idx.get(kmer)
            if idx is not None:
                acc += self.embeddings[idx]
                count += 1

        if count == 0:
            return np.zeros(self.dim)
        return (acc > count / 2).astype(np.float32)

    def encode_batch(self, sequences: List[str]) -> np.ndarray:
        return np.stack([self.encode(s) for s in sequences])


def knn_predict(train_vecs, train_labels, test_vecs, k=5):
    """k-NN prediction."""
    predictions = []
    for tv in test_vecs:
        if train_vecs.max() > 1:  # Continuous - cosine
            sims = train_vecs @ tv / (np.linalg.norm(train_vecs, axis=1) * np.linalg.norm(tv) + 1e-10)
        else:  # Binary - Hamming
            sims = np.mean(train_vecs == tv, axis=1)
        top_k = np.argsort(sims)[-k:]
        votes = [train_labels[j] for j in top_k]
        predictions.append(Counter(votes).most_common(1)[0][0])
    return np.array(predictions)


# =============================================================================
# DATA GENERATION
# =============================================================================

def generate_promoter_dataset(n_samples: int, seq_length: int = 100, seed: int = 42):
    """Generate synthetic promoter dataset with stronger signal."""
    rng = np.random.RandomState(seed)
    bases = ['A', 'C', 'G', 'T']
    sequences, labels = [], []

    tata_box = "TATAAA"
    caat_box = "CCAAT"
    gc_box = "GGGCGG"

    for i in range(n_samples):
        seq = list(''.join(rng.choice(bases, seq_length)))

        if i < n_samples // 2:
            # Promoter: insert TATA box
            pos = rng.randint(20, 30)
            for j, c in enumerate(tata_box):
                if pos + j < seq_length:
                    seq[pos + j] = c

            # Sometimes add CAAT box
            if rng.random() < 0.6:
                caat_pos = rng.randint(45, 55)
                for j, c in enumerate(caat_box):
                    if caat_pos + j < seq_length:
                        seq[caat_pos + j] = c

            # Sometimes add GC box
            if rng.random() < 0.3:
                gc_pos = rng.randint(60, 70)
                for j, c in enumerate(gc_box):
                    if gc_pos + j < seq_length:
                        seq[gc_pos + j] = c

            labels.append(1)
        else:
            labels.append(0)

        sequences.append(''.join(seq))

    idx = rng.permutation(len(sequences))
    return [sequences[i] for i in idx], [labels[i] for i in idx]


# =============================================================================
# MAIN EXPERIMENT
# =============================================================================

def run_full_pipeline(n_samples: int = 1000, seed: int = 42):
    """Run complete hybrid HDC pipeline."""
    print("\n" + "="*70)
    print("  HYBRID HDC: CONTRASTIVE PRE-TRAINING + LEARNED FINE-TUNING")
    print("="*70 + "\n", flush=True)

    # Generate data
    print("Generating dataset...", flush=True)
    sequences, labels = generate_promoter_dataset(n_samples, seed=seed)

    # Split: 60% train, 20% val, 20% test
    n = len(sequences)
    train_end = int(0.6 * n)
    val_end = int(0.8 * n)

    train_seqs, train_labels = sequences[:train_end], labels[:train_end]
    val_seqs, val_labels = sequences[train_end:val_end], labels[train_end:val_end]
    test_seqs, test_labels = sequences[val_end:], labels[val_end:]

    print(f"  Train: {len(train_seqs)}, Val: {len(val_seqs)}, Test: {len(test_seqs)}")

    config = HybridConfig(dim=HYPERVECTOR_DIM, kmer_length=6, num_classes=2)

    # =========================================================================
    # BASELINE
    # =========================================================================
    print("\n" + "-"*60)
    print("BASELINE: Random HDC + k-NN")
    print("-"*60, flush=True)

    baseline = BaselineHDC(config.dim, config.kmer_length)
    train_vecs = baseline.encode_batch(train_seqs)
    test_vecs = baseline.encode_batch(test_seqs)
    baseline_preds = knn_predict(train_vecs, train_labels, test_vecs)
    baseline_acc = np.mean(baseline_preds == np.array(test_labels))
    print(f"  Test Accuracy: {baseline_acc*100:.2f}%", flush=True)

    # =========================================================================
    # HYBRID MODEL
    # =========================================================================
    model = HybridHDCModel(config, seed=seed)

    # Stage 1: Contrastive pre-training (uses ALL sequences, no labels)
    all_seqs = train_seqs + val_seqs + test_seqs  # Can use unlabeled data!
    model.contrastive_pretrain(all_seqs, verbose=True)

    # Stage 2: Supervised fine-tuning
    finetune_result = model.finetune(
        train_seqs, train_labels,
        val_seqs, val_labels,
        verbose=True
    )

    # Final evaluation
    print("\n" + "-"*60)
    print("FINAL EVALUATION")
    print("-"*60, flush=True)

    test_preds = model.predict(test_seqs)
    test_acc = np.mean(test_preds == np.array(test_labels))
    print(f"  Hybrid HDC Test Accuracy: {test_acc*100:.2f}%", flush=True)

    # Also evaluate k-NN with learned embeddings
    train_vecs_learned = model.encode_batch(train_seqs)
    test_vecs_learned = model.encode_batch(test_seqs)
    knn_preds = knn_predict(train_vecs_learned, train_labels, test_vecs_learned)
    knn_acc = np.mean(knn_preds == np.array(test_labels))
    print(f"  Hybrid HDC k-NN Accuracy: {knn_acc*100:.2f}%", flush=True)

    # Summary
    print("\n" + "="*70)
    print("SUMMARY")
    print("="*70)
    print(f"\n  {'Method':<45} {'Accuracy':>12}")
    print(f"  {'-'*57}")
    print(f"  {'Baseline (Random HDC + k-NN)':<45} {baseline_acc*100:>11.2f}%")
    print(f"  {'Hybrid HDC (Classifier)':<45} {test_acc*100:>11.2f}%")
    print(f"  {'Hybrid HDC (k-NN)':<45} {knn_acc*100:>11.2f}%")

    improvement = (test_acc - baseline_acc) / baseline_acc * 100 if baseline_acc > 0 else 0
    print(f"\n  Improvement over baseline: {improvement:+.1f}%", flush=True)

    return {
        'baseline_acc': baseline_acc,
        'hybrid_classifier_acc': test_acc,
        'hybrid_knn_acc': knn_acc,
        'improvement': improvement,
        'best_val_acc': finetune_result['best_val_acc'],
        'best_epoch': finetune_result['best_epoch']
    }


if __name__ == '__main__':
    results = run_full_pipeline(n_samples=1000)

    print("\n" + "="*70)
    print("CONCLUSION")
    print("="*70)
    print(f"""
  The hybrid approach combines:
  1. Contrastive pre-training (learns from unlabeled data)
  2. Supervised fine-tuning (optimizes for task)
  3. Early stopping (prevents overfitting)

  Result: {results['improvement']:+.1f}% improvement over baseline

  This approach is ready for:
  - Validation on real genomic data
  - PyTorch GPU implementation
  - Integration into Rust hdc-core
""", flush=True)
