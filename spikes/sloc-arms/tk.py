# tokei JSON reducer for the VM-2 spike. Not TCB; not SLOC-counted.
import json, sys
mode = sys.argv[1]; d = json.load(sys.stdin); items = [(k, v["code"]) for k, v in d.items() if k != "Total" and v["code"] > 0]
if mode == "sum": print(sum(c for _, c in items))
elif mode == "lang": print(", ".join(f"{k}={c}" for k, c in sorted(items, key=lambda x: -x[1])))
elif mode == "unsafe": print(", ".join(f"{k}={c}" for k, c in items if k in ("C", "C Header", "C++", "Assembly", "GNU Style Assembly")) or "none")
