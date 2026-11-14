import csv
import sys
from pathlib import Path

BUDGET_MS = 16
MIN_SAMPLES = 20


def column(header, name):
    for index, cell in enumerate(header):
        if cell.strip() == name:
            return index
    return None


def numbers(rows, index):
    values = []
    for row in rows:
        if index >= len(row):
            continue
        cell = row[index].strip()
        if not cell or cell.upper() == "NA":
            continue
        values.append(float(cell))
    return values


def percentile(values, p):
    ordered = sorted(values)
    return ordered[max(0, int(len(ordered) * p + 0.999999) - 1)]


def main():
    path = Path(sys.argv[1])
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle)
        header = next(reader)
        rows = list(reader)
    latency_index = column(header, "MsAllInputToPhotonLatency")
    frame_index = column(header, "MsBetweenPresents")
    if latency_index is None or frame_index is None:
        print("header " + ",".join(header))
        print("PRESENTMON_COLUMNS_MISSING")
        return 1
    latency = numbers(rows, latency_index)
    frames = numbers(rows, frame_index)
    print(
        f"input-to-photon n={len(latency)} p50={percentile(latency, 0.5):.3f} p95={percentile(latency, 0.95):.3f}"
        if latency
        else "input-to-photon n=0"
    )
    print(
        f"frame-interval n={len(frames)} p50={percentile(frames, 0.5):.3f} p95={percentile(frames, 0.95):.3f}"
        if frames
        else "frame-interval n=0"
    )
    if len(latency) < MIN_SAMPLES or len(frames) < MIN_SAMPLES:
        print("PRESENTMON_TOO_FEW_SAMPLES")
        return 1
    if percentile(latency, 0.95) >= BUDGET_MS:
        print("PRESENTMON_INPUT_BUDGET_MISS")
        return 1
    print("PRESENTMON_OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
