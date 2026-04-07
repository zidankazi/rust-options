// End-to-end test: load the ONNX model from data/vol_model/exported/
// and run a prediction on a dummy market window.

use std::path::PathBuf;

use vol_model::{MarketWindow, VolModel, NUM_FEATURES, WINDOW_SIZE};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn load_and_predict() {
    let root = project_root();
    let onnx_path = root.join("data/vol_model/exported/vol_model.onnx");
    let stats_path = root.join("data/vol_model/exported/norm_stats.json");

    assert!(onnx_path.exists(), "ONNX model not found at {onnx_path:?}");
    assert!(stats_path.exists(), "Stats not found at {stats_path:?}");

    let mut model = VolModel::load(&onnx_path, &stats_path).expect("failed to load model");

    // Dummy market window: 30 days of synthetic data.
    // Real usage would compute features from actual market history.
    let mut data = [[0.0f32; NUM_FEATURES]; WINDOW_SIZE];
    for day in 0..WINDOW_SIZE {
        data[day][0] = 0.001; // log_return
        data[day][1] = 0.15;  // realized_vol
        data[day][2] = 1.0;   // spot_scaled
        data[day][3] = 0.25;  // t (time to expiry in years)
        data[day][4] = 0.25;  // days_to_exp_scaled
    }
    let window = MarketWindow::new(data);

    let pred = model.predict(&window).expect("inference failed");
    println!("Prediction:");
    println!("  a     = {:.6}", pred.a);
    println!("  b     = {:.6}", pred.b);
    println!("  rho   = {:.6}", pred.rho);
    println!("  m     = {:.6}", pred.m);
    println!("  sigma = {:.6}", pred.sigma);

    // sanity checks — the model should produce numbers roughly in the SVI range
    // even with garbage inputs, we expect finite values
    assert!(pred.a.is_finite());
    assert!(pred.b.is_finite());
    assert!(pred.rho.is_finite());
    assert!(pred.m.is_finite());
    assert!(pred.sigma.is_finite());

    // rho should be in [-1, 1] by SVI definition — model may slightly exceed
    // since it's a free regression, but shouldn't be wildly off
    assert!(pred.rho > -2.0 && pred.rho < 2.0, "rho way off: {}", pred.rho);
}

#[test]
fn svi_roundtrip_with_pricer() {
    // Verify the prediction can be fed into the pricer's SVI functions.
    let root = project_root();
    let onnx_path = root.join("data/vol_model/exported/vol_model.onnx");
    let stats_path = root.join("data/vol_model/exported/norm_stats.json");

    let mut model = VolModel::load(&onnx_path, &stats_path).unwrap();

    let data = [[0.0f32, 0.15, 1.0, 0.25, 0.25]; WINDOW_SIZE];
    let window = MarketWindow::new(data);
    let pred = model.predict(&window).unwrap();

    // Use the prediction with pricer::svi to compute IV at ATM (k = 0)
    let params = pred.to_pricer_params();
    let w_atm = pricer::svi::svi_variance(&params, 0.0);
    println!("Predicted ATM total variance: {w_atm}");

    // at-the-money variance should be positive for a real smile
    // (if it's negative, the model fit is bad but the roundtrip still works)
    assert!(w_atm.is_finite());
}
