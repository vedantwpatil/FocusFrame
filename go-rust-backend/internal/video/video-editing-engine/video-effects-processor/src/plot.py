import matplotlib.pyplot as plt
import pandas as pd

control_points = pd.read_csv("control_points.csv")
spline_points = pd.read_csv("spline_points.csv")

plt.figure(figsize=(8, 6))
plt.plot(spline_points["x"], spline_points["y"], label="Catmull-Rom Spline", marker=".")
plt.scatter(
    control_points["x"], control_points["y"], color="red", label="Control Points"
)
plt.title("Catmull-Rom Spline Visualization")
plt.xlabel("X")
plt.ylabel("Y")
plt.legend()
plt.grid(True)
plt.show()
