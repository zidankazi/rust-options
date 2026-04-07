# Training loop for the vol model transformer.
# Loads train/val datasets, runs MSE training, saves the best checkpoint by val loss.

import time
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
from torch.utils.data import DataLoader

from dataset import VolDataset
from model import VolTransformer

CHECKPOINT_DIR = Path(__file__).resolve().parent / "checkpoints"
CHECKPOINT_DIR.mkdir(exist_ok=True)

# hyperparameters
BATCH_SIZE = 32
LEARNING_RATE = 1e-3
NUM_EPOCHS = 100
WEIGHT_DECAY = 1e-4  # L2 regularization — helps with overfitting
SEED = 42


def set_seed(seed: int):
    """Make training reproducible."""
    torch.manual_seed(seed)
    np.random.seed(seed)


def train_one_epoch( # how the model learns and updates its weights
    model: nn.Module,
    loader: DataLoader,
    optimizer: torch.optim.Optimizer,
    loss_fn: nn.Module,
) -> float:
    """Run one pass over the training set. Returns mean loss."""
    model.train()  # enables dropout, batchnorm updates, etc.
    total_loss = 0.0
    num_batches = 0

    for x, y in loader:
        pred = model(x) # [batch, 5] - predicted svi parameters
        loss = loss_fn(pred, y) # [batch, 5] - loss for each example

        optimizer.zero_grad() # clear old gradients
        # gradients are accumulated by default, so we need to clear them before backpropagation
        
        loss.backward() # backpropagation - compute gradients of the loss with respect to the model's parameters
        optimizer.step() # update weights - update the model's weights based on the gradients
        
        total_loss += loss.item() # accumulate loss in order to compute the mean loss, which is the loss for the entire epoch
        # mean loss is needed to track the model's performance over time, which is used to determine when to stop training

        num_batches += 1

    return total_loss / max(num_batches, 1)


@torch.no_grad()  # disables gradient tracking — faster, no memory for backprop
def evaluate( # measuring whether the learning is working
    model: nn.Module,
    loader: DataLoader,
    loss_fn: nn.Module,
) -> float:
    """Run one pass over a val/test set. Returns mean loss. No weight updates."""
    model.eval()  # disables dropout
    total_loss = 0.0
    num_batches = 0

    for x, y in loader:
        pred = model(x) # [batch, 5] - predicted svi parameters
        loss = loss_fn(pred, y) # [batch, 5] - loss for each example

        total_loss += loss.item() # accumulate loss in order to compute the mean loss, which is the loss for the entire epoch
        # mean loss is needed to track the model's performance over time, which is used to determine when to stop training

        num_batches += 1

    return total_loss / max(num_batches, 1)


def main():
    set_seed(SEED)
    print(f"=== Vol Model Training ===")
    print(f"batch_size={BATCH_SIZE}  lr={LEARNING_RATE}  epochs={NUM_EPOCHS}")
    print()

    # datasets
    train_ds = VolDataset("train")
    val_ds = VolDataset("val")
    # propagate normalization stats from train → val
    val_ds.set_norm_stats(
        train_ds.feat_mean, train_ds.feat_std,
        train_ds.label_mean, train_ds.label_std,
    )

    # dataloaders
    train_loader = DataLoader(train_ds, batch_size=BATCH_SIZE, shuffle=True)
    val_loader = DataLoader(val_ds, batch_size=BATCH_SIZE, shuffle=False)

    # model + optimizer + loss
    model = VolTransformer(
        num_features=5,
        window_size=30,
        d_model=16,
        num_heads=2,
        num_layers=1,
        dropout=0.1,
    )
    num_params = sum(p.numel() for p in model.parameters())
    print(f"model parameters: {num_params:,}")
    print()

    optimizer = torch.optim.Adam(
        model.parameters(),
        lr=LEARNING_RATE,
        weight_decay=WEIGHT_DECAY,
    )
    loss_fn = nn.MSELoss()

    # training loop
    best_val_loss = float("inf")
    best_epoch = 0

    for epoch in range(1, NUM_EPOCHS + 1):
        t0 = time.time()
        train_loss = train_one_epoch(model, train_loader, optimizer, loss_fn)
        val_loss = evaluate(model, val_loader, loss_fn)
        dt = time.time() - t0

        marker = ""
        if val_loss < best_val_loss:
            best_val_loss = val_loss
            best_epoch = epoch
            marker = " ← new best"
            # save checkpoint + norm stats (needed for inference later)
            torch.save(
                {
                    "model_state": model.state_dict(),
                    "feat_mean": train_ds.feat_mean,
                    "feat_std": train_ds.feat_std,
                    "label_mean": train_ds.label_mean,
                    "label_std": train_ds.label_std,
                    "epoch": epoch,
                    "val_loss": val_loss,
                },
                CHECKPOINT_DIR / "best.pt",
            )

        print(
            f"epoch {epoch:3d}  "
            f"train_loss={train_loss:.6f}  "
            f"val_loss={val_loss:.6f}  "
            f"({dt:.1f}s){marker}"
        )

    print()
    print(f"best val_loss={best_val_loss:.6f} at epoch {best_epoch}")
    print(f"checkpoint saved to {CHECKPOINT_DIR / 'best.pt'}")


if __name__ == "__main__":
    main()
