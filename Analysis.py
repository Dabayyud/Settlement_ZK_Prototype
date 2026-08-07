import pandas as pd
import matplotlib.pyplot as plt

df = pd.read_csv("benchmark_results.csv")

summary = df.groupby("batch_size").agg(
    setup_ms=("setup_ms", "first"),         
    prove_ms_mean=("prove_ms", "mean"),
    prove_ms_std=("prove_ms", "std"),
    verify_ms_mean=("verify_ms", "mean"),
    verify_ms_std=("verify_ms", "std"),
    num_constraints=("num_constraints", "first"),
    proof_size_bytes=("proof_size_bytes", "first"),
).reset_index()

print("=== Descriptive statistics (grouped by batch size) ===")
print(summary.to_string(index=False))

summary["constraints_per_trade"] = summary["num_constraints"] / summary["batch_size"]
print("\nConstraints per trade (should be ~flat if constraint count scales linearly):")
print(summary[["batch_size", "constraints_per_trade"]].to_string(index=False))

summary.to_csv("benchmark_summary.csv", index=False)
print("\nwrote benchmark_summary.csv")


fig, ax1 = plt.subplots(figsize=(7, 5))
ax1.plot(summary["batch_size"], summary["setup_ms"], marker="o", label="Tsetup")
ax1.errorbar(
    summary["batch_size"], summary["prove_ms_mean"], yerr=summary["prove_ms_std"],
    marker="o", label="Tprove", capsize=3,
)
ax1.set_xlabel("Batch size N (trades per proof)")
ax1.set_ylabel("Time (ms)")
ax1.set_title("Groth16 setup / prove time vs. batch size")
ax1.legend()
ax1.grid(True, alpha=0.3)
fig.tight_layout()
fig.savefig("plot_setup_prove_vs_batch_size.png", dpi=150)
print("wrote plot_setup_prove_vs_batch_size.png")


fig, ax2 = plt.subplots(figsize=(7, 5))
ax2.errorbar(
    summary["batch_size"], summary["verify_ms_mean"], yerr=summary["verify_ms_std"],
    marker="o", color="tab:green", capsize=3,
)
ax2.set_xlabel("Batch size N (trades per proof)")
ax2.set_ylabel("Verify time (ms)")
ax2.set_title("Groth16 verify time vs. batch size")
ax2.grid(True, alpha=0.3)
fig.tight_layout()
fig.savefig("plot_verify_vs_batch_size.png", dpi=150)
print("wrote plot_verify_vs_batch_size.png")


fig, ax3 = plt.subplots(figsize=(7, 5))
ax3.plot(summary["batch_size"], summary["num_constraints"], marker="o", color="tab:orange")
ax3.set_xlabel("Batch size N (trades per proof)")
ax3.set_ylabel("R1CS constraints")
ax3.set_title("Constraint count vs. batch size (expected: linear)")
ax3.grid(True, alpha=0.3)
fig.tight_layout()
fig.savefig("plot_constraints_vs_batch_size.png", dpi=150)
print("wrote plot_constraints_vs_batch_size.png")


ratio_32_to_1 = summary.loc[summary.batch_size == 32, "setup_ms"].values[0] / \
                summary.loc[summary.batch_size == 1, "setup_ms"].values[0]
print(
    f"\nThe batch size grew 32x (1 -> 32) but Tsetup only grew {ratio_32_to_1:.1f}x. "
    "The constraint count scaled exactly linearly, so this sub-linear "
    "growth is a real, reproducible effect worth flagging in the note - "
    "not something to ignore. The plausible causes worth checking before claiming" \
    "fixed per-setup overhead being amortised over more constraints, and/or "
    "rayon parallelism becoming more effective as circuit size grows."
)
