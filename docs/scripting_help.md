# PlotRedox — Scripting Reference

The scripting engine uses **Rhai** (a Rust-embedded scripting language).
Its syntax is similar to Rust / JavaScript. This guide covers the essentials.

---

## 1. Variables

```rhai
let x = 42;            // integer (i64)
let y = 3.14;          // float   (f64)
let name = "hello";    // string
let flag = true;       // boolean
```

- Use `let` to declare variables (required, unlike Python).
- Variables are **mutable** by default.
- Use `const` for constants: `const PI_2 = PI() / 2.0;`

---

## 2. Print & String Interpolation

```rhai
print("hello world");
print(`x = ${x}, y = ${y}`);   // backtick strings support interpolation
```

---

## 3. Operators

| Operator | Meaning |
|----------|---------|
| `+  -  *  /` | Arithmetic |
| `%` | Modulus |
| `==  !=  <  >  <=  >=` | Comparison |
| `&&  \|\|  !` | Logical AND, OR, NOT |
| `+=  -=  *=  /=` | Compound assignment |

> **Note**: Integer ÷ integer = integer (truncates). Use `.0` for float division: `7.0 / 2.0`

---

## 4. Control Flow

```rhai
if x > 0 {
    print("positive");
} else if x == 0 {
    print("zero");
} else {
    print("negative");
}

// For loop over array
for item in [1, 2, 3] {
    print(item);
}

// While loop
let i = 0;
while i < 10 {
    i += 1;
}
```

Use `continue` and `break` inside loops.

---

## 5. Arrays & Maps

```rhai
let arr = [1, 2, 3, 4, 5];
arr.push(6);
let length = arr.len();

let map = #{};            // empty map (object)
map.name = "test";
map.value = 42;
// or: let map = #{ name: "test", value: 42 };

let keys = map.keys();   // ["name", "value"]
```

---

## 6. Data Access

Your digitized data is available as a global `data` map:

```rhai
// data is a Map: group_name -> Array of point maps
// Each point: #{ x, y, px, py }
//   x, y  = calibrated (logical) coordinates
//   px, py = pixel coordinates on the image

for name in data.keys() {
    let pts = data[name];
    print(`${name}: ${pts.len()} points`);

    if pts.len() > 0 {
        print(`  first point: (${pts[0].x}, ${pts[0].y})`);
    }
}
```

Use `col()` to extract a column from an array of maps:
```rhai
let xs = col(data["Group 1"], "x");  // -> array of x values
let ys = col(data["Group 1"], "y");
```

---

## 7. Available Functions

### Math (scalar)

| Function | Description | Example |
|----------|-------------|---------|
| `abs(x)` | Absolute value | `abs(-3.0)` -> `3.0` |
| `sqrt(x)` | Square root | `sqrt(9.0)` -> `3.0` |
| `ln(x)` | Natural logarithm | `ln(exp(1.0))` -> `1.0` |
| `log10(x)` | Base-10 logarithm | `log10(100.0)` -> `2.0` |
| `log2(x)` | Base-2 logarithm | `log2(8.0)` -> `3.0` |
| `exp(x)` | e^x | `exp(0.0)` -> `1.0` |
| `pow(x, y)` | x^y | `pow(2.0, 3)` -> `8.0` |
| `pow10(x)` | 10^x | `pow10(2.0)` -> `100.0` |
| `sin(x)` `cos(x)` `tan(x)` | Trigonometric | Radians |
| `asin(x)` `acos(x)` `atan(x)` | Inverse trig | Returns radians |
| `atan2(y, x)` | 2-argument arctangent | |
| `floor(x)` `ceil(x)` `round(x)` | Rounding | |
| `round_to(x, y)`| Rounding to | `round_to(2.3333, 2)` -> `2.33` |
| `PI()` | π constant | `3.14159…` |

### Array Aggregation

| Function | Description | Example |
|----------|-------------|---------|
| `sum(arr)` | Sum of elements | `sum([1,2,3])` -> `6` |
| `mean(arr)` | Arithmetic mean | `mean([2,4])` -> `3` |
| `min_val(arr)` | Minimum value | `min_val([3,1,2])` -> `1` |
| `max_val(arr)` | Maximum value | `max_val([3,1,2])` -> `3` |
| `std_dev(arr)` | Standard deviation | (sample, n−1) |
| `variance(arr)` | Variance | (sample, n−1) |

### Array Operations

| Function | Description | Example |
|----------|-------------|---------|
| `log10_array(arr)` | Element-wise log₁₀ | `log10_array([10, 100])` -> `[1, 2]` |

### Data Helpers

| Function | Description | Example |
|----------|-------------|---------|
| `col(arr, "field")` | Extract field from array of maps | `col(pts, "x")` |
| `extract_number(s)` | First number in a string | `extract_number("20kHz")` -> `20.0` |

### Regression & Fitting

| Function | Returns | Description |
|----------|---------|-------------|
| `linreg(xs, ys)` | `#{ slope, intercept, r_squared }` | Linear regression |
| `polyfit(xs, ys, deg)` | `#{ coeffs: [...], r_squared }` | Polynomial fit (degree 1–N) |
| `lstsq(A, b)` | `[c0, c1, ...]` | Least-squares solve A·c = b (A = 2D array) |

#### Example — Linear Regression
```rhai
let xs = col(data["Group 1"], "x");
let ys = col(data["Group 1"], "y");
let fit = linreg(xs, ys);
print(`slope = ${fit.slope}, R² = ${fit.r_squared}`);
```

#### Example — Polynomial Fit
```rhai
let fit = polyfit(xs, ys, 3);  // cubic
for i in 0..fit.coeffs.len() {
    print(`  a${i} = ${fit.coeffs[i]}`);
}
```

#### Example — Least-Squares (multi-variate)
```rhai
// Solve: y = c0 + c1*x1 + c2*x2
let A = [];
let b = [];
// ... fill A with rows [1.0, x1_i, x2_i] and b with y_i ...
let coeffs = lstsq(A, b);
```

---

## 8. Tips

- Integer math truncates: `7 / 2` = `3`. Use `7.0 / 2.0` for float division.
- String interpolation requires **backticks**: `` `value = ${x}` ``
- The `data` map is read-only; your script cannot modify the digitized points.
- All output from `print()` appears in the OUTPUT panel on the right.
- Variables created by your script appear in the WORKSPACE panel on the left — click to inspect.
