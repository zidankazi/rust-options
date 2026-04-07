# Exports the trained vol model to ONNX format for use from Rust via the ort crate.
# Loads the best checkpoint, runs a dummy input through the model, and saves
# the resulting ONNX file alongside the normalization stats needed for inference.

import json
from pathlib import Path

import numpy as np
import torch

from model import VolTransformer

CHECKPOINT_DIR = Path(__file__).resolve().parent / "checkpoints"
EXPORT_DIR = Path(__file__).resolve().parent / "exported"
EXPORT_DIR.mkdir(exist_ok=True)

CHECKPOINT_PATH = CHECKPOINT_DIR / "best.pt"
ONNX_PATH = EXPORT_DIR / "vol_model.onnx"
STATS_PATH = EXPORT_DIR / "norm_stats.json"

# must match train.py exactly — these are the architecture hyperparameters
# that define the model's shape
MODEL_CONFIG = dict(
    num_features=5,
    window_size=30,
    d_model=16,
    num_heads=2,
    num_layers=1,
    dropout=0.1,
)


def main():
    if not CHECKPOINT_PATH.exists():
        raise FileNotFoundError(
            f"No checkpoint at {CHECKPOINT_PATH}. Run train.py first."
        )

    print("=== Exporting vol model to ONNX ===")
    print()

    # load checkpoint
    ckpt = torch.load(CHECKPOINT_PATH, weights_only=False)
    print(f"loaded checkpoint from epoch {ckpt['epoch']}  val_loss={ckpt['val_loss']:.6f}")

    # rebuild the model and load weights
    model = VolTransformer(**MODEL_CONFIG)
    model.load_state_dict(ckpt["model_state"])
    model.eval()  # inference mode — disables dropout

    # dummy input matching the expected shape [batch, window, features]
    # batch dimension can be dynamic at inference time
    dummy_input = torch.randn(1, MODEL_CONFIG["window_size"], MODEL_CONFIG["num_features"])

    # sanity check: model still runs
    with torch.no_grad():
        out = model(dummy_input)
    print(f"dummy forward pass: input {tuple(dummy_input.shape)} → output {tuple(out.shape)}")

    # export to ONNX
    # dynamic_axes lets the batch dimension vary at runtime
    torch.onnx.export(
        model,
        dummy_input,
        ONNX_PATH,
        input_names=["market_window"],
        output_names=["svi_params"],
        dynamic_axes={
            "market_window": {0: "batch"},
            "svi_params": {0: "batch"},
        },
        opset_version=17,
    )
    print(f"saved ONNX to {ONNX_PATH}")

    # save normalization stats as JSON — Rust needs these to preprocess inputs
    # and denormalize outputs at inference time
    stats = {
        "feat_mean": ckpt["feat_mean"].tolist(),
        "feat_std": ckpt["feat_std"].tolist(),
        "label_mean": ckpt["label_mean"].tolist(),
        "label_std": ckpt["label_std"].tolist(),
        "window_size": MODEL_CONFIG["window_size"],
        "num_features": MODEL_CONFIG["num_features"],
    }
    with open(STATS_PATH, "w") as f:
        json.dump(stats, f, indent=2)
    print(f"saved norm stats to {STATS_PATH}")

    # verify the ONNX file by loading it back and running the same dummy input
    print()
    print("verifying ONNX roundtrip...")
    try:
        import onnx
        import onnxruntime as ort
    except ImportError:
        print("  onnxruntime not installed — skipping verification")
        print("  install with: pip install onnxruntime")
        return

    onnx_model = onnx.load(ONNX_PATH)
    onnx.checker.check_model(onnx_model)
    print(f"  onnx file is valid")

    session = ort.InferenceSession(str(ONNX_PATH))
    onnx_out = session.run(
        ["svi_params"],
        {"market_window": dummy_input.numpy().astype(np.float32)},
    )[0]
    torch_out = out.numpy()

    max_diff = np.abs(onnx_out - torch_out).max()
    print(f"  max difference (onnx vs pytorch): {max_diff:.2e}")
    if max_diff < 1e-5:
        print("  ✓ onnx output matches pytorch")
    else:
        print("  ✗ outputs differ — something's wrong with the export")

    print()
    print("=== Export complete ===")
    print(f"ONNX model:      {ONNX_PATH}")
    print(f"Norm stats:      {STATS_PATH}")
    print()
    print("Next step: load from Rust using the ort crate")


if __name__ == "__main__":
    main()
