# Transformer encoder for predicting SVI vol surface parameters.
# Takes a window of recent market days, outputs the 5 SVI knobs.
# Architecture: input projection → positional encoding → transformer blocks → pool → head.

import torch
import torch.nn as nn


class VolTransformer(nn.Module):
    def __init__(
        self,
        num_features: int = 5,   # number of features per day (spot, days_to_exp, etc.)
        window_size: int = 30,   # number of days of history to look at
        d_model: int = 64,       # number of dimensions of the model
        num_heads: int = 4,      # number of attention heads per block
        num_layers: int = 2,     # number of transformer blocks
        dropout: float = 0.1,    # regularization to prevent overfitting
    ):
        super().__init__()
        self.window_size = window_size
        self.d_model = d_model

        # 1. input projection: turn raw features into d_model-dim vectors
        # d_model-dim vectors refers to a vector with d_model number of dimensions
        self.input_proj = nn.Linear(num_features, d_model) 
        # output = x @ W + b, where W is a weight matrix and b is a bias vector

        # 2. positional encoding: a learned embedding, one vector per position
        self.pos_embedding = nn.Parameter(torch.zeros(window_size, d_model))

        # 3. transformer encoder blocks (using PyTorch built-in)
        # batch_first=True means input shape is [batch, seq, features]
        encoder_layer = nn.TransformerEncoderLayer(
            d_model=d_model,
            nhead=num_heads,
            dim_feedforward=d_model * 4,
            dropout=dropout,
            batch_first=True,
            activation="gelu",
        )
        self.transformer = nn.TransformerEncoder(encoder_layer, num_layers=num_layers)

        # 4. output head: turn the pooled vector into 5 SVI params
        # small MLP: d_model -> d_model -> 5
        self.head = nn.Sequential(
            nn.Linear(d_model, d_model),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(d_model, 5),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """
        x: [batch, window_size, num_features]
        returns: [batch, 5] — predicted SVI params (a, b, rho, m, sigma)
        """

        # 1. project the input features up to d_model dimensions
        x = self.input_proj(x) # shape [batch, window, d_model]

        # 2. add positional encoding to every day
        x = x + self.pos_embedding # shape [batch, window, d_model]

        # 3. run through transformer blocks
        x = self.transformer(x) # shape [batch, window, d_model]

        # 4. pool: take the last day's vector
        x = x[:, -1, :] # shape [batch, d_model]

        # 5. run through the output head
        return self.head(x) # shape [batch, 5]


if __name__ == "__main__":
    # create a model and run a dummy batch through it
    model = VolTransformer()
    print(model)

    # dummy input: batch of 8, 30 days, 5 features each
    dummy = torch.randn(8, 30, 5)
    out = model(dummy)
    print(f"\nInput shape:  {dummy.shape}")
    print(f"Output shape: {out.shape}")
    print(f"Expected:     torch.Size([8, 5])")
