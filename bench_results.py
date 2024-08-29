import re
import sys

def convert_to_ms(value, unit):
    if unit == 'µs':
        return value / 1000
    elif unit == 'ms':
        return value
    elif unit == 's':
        return value * 1000
    else:
        raise ValueError(f"Unknown time unit: {unit}")

def parse_benchmark_file(file_path):
    machines = {}
    pattern = r'(\w+)/\w+\s+time:\s+\[\d+(?:\.\d+)?\s(\w+)\s(\d+(?:\.\d+)?)\s(\w+)\s\d+(?:\.\d+)?\s(\w+)\]'

    with open(file_path, 'r') as file:
        content = file.read()
        # Clean the file content
        cleaned_content = re.sub(r'\n(\s+)time:', ' time:', content)
        for line in cleaned_content.split('\n'):
            match = re.match(pattern, line)
            if match:
                machine, lower_unit, mid_time, mid_unit, upper_unit = match.groups()
                # Convert mid_time to milliseconds
                mid_time_ms = convert_to_ms(float(mid_time), mid_unit)
                machines[machine] = machines.get(machine, 0) + mid_time_ms

    return machines

def main():
    if len(sys.argv) != 2:
        print("Usage: results.py <path-to-file>")
        return
    file_path = sys.argv[1]
    results = parse_benchmark_file(file_path)

    for machine, total_time in results.items():
        print(f"Total {machine} time: {total_time:.3f} ms")

    if not "lambda" in results:
        return

    print("")

    if "legacy" in results:
        lambda_vs_legacy = round(results["lambda"] / results["legacy"], 1)
        print(f"lambda_vm took x{lambda_vs_legacy} more than legacy_vm")
    if "fast" in results:
        lambda_vs_fast = round(results["lambda"] / results["fast"], 1)
        print(f"lambda_vm took x{lambda_vs_fast} more than fast_vm")

if __name__ == "__main__":
    main()
