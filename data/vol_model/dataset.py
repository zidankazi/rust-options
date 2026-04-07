# PyTorch Dataset for the vol model.
# Loads svi_dataset.csv, builds 30-day market windows for each row,
# and pairs each window with its 5 SVI parameter labels.

import json
import datetime
from pathlib import Path

import numpy as np
import pandas as pd
import torch
from torch.utils.data import Dataset

DATA_DIR = Path(__file__).resolve().parent.parent
SVI_CSV = DATA_DIR / "svi_dataset.csv"
SPY_JSON = DATA_DIR / "raw" / "spy_daily.json"

WINDOW_SIZE = 30
NUM_FEATURES = 5
SVI_COLS = ["svi_a", "svi_b", "svi_rho", "svi_m", "svi_sigma"]


def load_spot_history() -> pd.DataFrame:
    """Load SPY daily bars as a DataFrame indexed by date."""
    with open(SPY_JSON) as f:
        bars = json.load(f)

    rows = []
    for bar in bars:
        ts = bar["t"] / 1000
        date = datetime.date.fromtimestamp(ts)
        rows.append({"date": date, "close": bar["c"]})

    df = pd.DataFrame(rows)
    df = df.sort_values("date").reset_index(drop=True)
    # compute derived features
    df["log_return"] = np.log(df["close"] / df["close"].shift(1)).fillna(0.0)
    # 5-day rolling realized vol (annualized)
    df["realized_vol"] = df["log_return"].rolling(5).std().fillna(0.0) * np.sqrt(252)
    return df


class VolDataset(Dataset):
    """Each example is (30-day market window, 5 SVI labels)."""

    def __init__(self, split: str = "train"):
        self.split = split
        self.window_size = WINDOW_SIZE

        # load raw data
        svi_df = pd.read_csv(SVI_CSV)
        svi_df["date"] = pd.to_datetime(svi_df["date"]).dt.date

        spot_df = load_spot_history()
        # build a date → row index lookup for fast window slicing
        self.spot_df = spot_df
        self.date_to_idx = {d: i for i, d in enumerate(spot_df["date"])}

        # filter svi rows to those where we have a full 30-day window of spot history
        valid_rows = []
        for _, row in svi_df.iterrows():
            d = row["date"]
            if d not in self.date_to_idx:
                continue
            idx = self.date_to_idx[d]
            if idx < WINDOW_SIZE - 1:
                continue  # not enough history
            valid_rows.append(row)
        svi_df = pd.DataFrame(valid_rows).reset_index(drop=True)

        # chronological split: 70% train, 15% val, 15% test
        svi_df = svi_df.sort_values("date").reset_index(drop=True)
        n = len(svi_df)
        train_end = int(n * 0.70)
        val_end = int(n * 0.85)

        if split == "train":
            self.rows = svi_df.iloc[:train_end].reset_index(drop=True)
        elif split == "val":
            self.rows = svi_df.iloc[train_end:val_end].reset_index(drop=True)
        elif split == "test":
            self.rows = svi_df.iloc[val_end:].reset_index(drop=True)
        else:
            raise ValueError(f"unknown split: {split}")

        # compute normalization stats from the training split only,
        # to avoid leaking future info into train/val/test
        if split == "train":
            self._compute_norm_stats()
        else:
            # val/test reuse the train stats (set by the train instance)
            pass

        print(f"[{split}] {len(self.rows)} examples")

    def _compute_norm_stats(self):
        """Compute mean/std of each feature on the training set for normalization."""
        # gather all feature vectors across training windows, then compute stats
        all_features = []
        all_labels = []
        for i in range(len(self.rows)):
            feats, labels = self._build_example(i, normalize=False)
            all_features.append(feats)
            all_labels.append(labels)

        all_features = np.stack(all_features)  # [N, window, num_features]
        all_labels = np.stack(all_labels)      # [N, 5]

        # feature stats: one mean/std per feature dim
        self.feat_mean = all_features.mean(axis=(0, 1))  # [num_features]
        self.feat_std = all_features.std(axis=(0, 1)) + 1e-6

        # label stats: one mean/std per SVI param
        self.label_mean = all_labels.mean(axis=0)  # [5]
        self.label_std = all_labels.std(axis=0) + 1e-6

    def set_norm_stats(self, feat_mean, feat_std, label_mean, label_std):
        """Propagate train-set stats to val/test instances."""
        self.feat_mean = feat_mean
        self.feat_std = feat_std
        self.label_mean = label_mean
        self.label_std = label_std

    def _build_example(self, i: int, normalize: bool = True):
        """Build one (features, labels) pair for row i.

        Features: 30-day window of [log_return, realized_vol, spot_scaled, t, days_to_exp_scaled]
        Labels: the 5 SVI params for this row
        """
        row = self.rows.iloc[i]
        d = row["date"]
        end_idx = self.date_to_idx[d]
        start_idx = end_idx - WINDOW_SIZE + 1

        # grab 30 days of spot history ending on row's date (inclusive)
        window = self.spot_df.iloc[start_idx:end_idx + 1]

        log_return = window["log_return"].values # log return of the past 30 days
        realized_vol = window["realized_vol"].values # 5-day rolling realized vol (annualized)
        spot_scaled = window["close"].values / row["spot"] # spot price scaled by the target day's spot price
        t = np.full(WINDOW_SIZE, row["t"]) # time to expiry in years (same for all 30 days)
        days_to_exp_scaled = np.full(WINDOW_SIZE, row["days_to_exp"] / 365.0) # days to expiry scaled by 365
        features = np.column_stack((log_return, realized_vol, spot_scaled, t, days_to_exp_scaled))
        features = features.astype(np.float32)

        # labels: 5 SVI params as float32 array
        labels = row[SVI_COLS].values.astype(np.float32)

        if normalize:
            features = (features - self.feat_mean) / self.feat_std
            labels = (labels - self.label_mean) / self.label_std

        return features, labels

    def __len__(self) -> int:
        return len(self.rows)

    def __getitem__(self, i: int):
        features, labels = self._build_example(i, normalize=True)
        return torch.from_numpy(features), torch.from_numpy(labels)


if __name__ == "__main__":
    # sanity check: build train/val/test, print shapes, show one example
    train = VolDataset("train")
    val = VolDataset("val")
    val.set_norm_stats(train.feat_mean, train.feat_std, train.label_mean, train.label_std)
    test = VolDataset("test")
    test.set_norm_stats(train.feat_mean, train.feat_std, train.label_mean, train.label_std)

    print(f"\nTrain stats:")
    print(f"  feature mean: {train.feat_mean}")
    print(f"  feature std:  {train.feat_std}")
    print(f"  label mean:   {train.label_mean}")
    print(f"  label std:    {train.label_std}")

    # grab one example
    x, y = train[0]
    print(f"\nExample 0:")
    print(f"  features shape: {x.shape}  (expected [30, 5])")
    print(f"  labels shape:   {y.shape}  (expected [5])")
    print(f"  features dtype: {x.dtype}")
    print(f"  labels dtype:   {y.dtype}")
