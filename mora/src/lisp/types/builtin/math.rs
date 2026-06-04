use super::register_native;
use crate::lisp::ns::Namespace;
use crate::lisp::types::Value;

pub fn register(ns: &mut Namespace) {
    // Constants — registered as direct values, not functions
    ns.intern("mora.math/PI", Value::Float(std::f64::consts::PI));
    ns.intern("mora.math/E", Value::Float(std::f64::consts::E));
    ns.intern("mora.math/TAU", Value::Float(std::f64::consts::TAU));
    ns.intern("mora.math/INFINITY", Value::Float(f64::INFINITY));
    ns.intern("mora.math/NAN", Value::Float(f64::NAN));

    // Basic
    register_native(ns, "mora.math/abs", native_math_abs);
    register_native(ns, "mora.math/signum", native_signum);
    register_native(ns, "mora.math/round", native_round);
    register_native(ns, "mora.math/floor", native_floor);
    register_native(ns, "mora.math/ceil", native_ceil);
    register_native(ns, "mora.math/ulp", native_ulp);

    // Exponent / logarithm
    register_native(ns, "mora.math/pow", native_pow);
    register_native(ns, "mora.math/exp", native_exp);
    register_native(ns, "mora.math/log", native_log);
    register_native(ns, "mora.math/log10", native_log10);
    register_native(ns, "mora.math/log2", native_log2);
    register_native(ns, "mora.math/sqrt", native_sqrt);
    register_native(ns, "mora.math/cbrt", native_cbrt);
    register_native(ns, "mora.math/hypot", native_hypot);
    register_native(ns, "mora.math/expm1", native_expm1);
    register_native(ns, "mora.math/log1p", native_log1p);

    // Trigonometric
    register_native(ns, "mora.math/sin", native_sin);
    register_native(ns, "mora.math/cos", native_cos);
    register_native(ns, "mora.math/tan", native_tan);
    register_native(ns, "mora.math/asin", native_asin);
    register_native(ns, "mora.math/acos", native_acos);
    register_native(ns, "mora.math/atan", native_atan);
    register_native(ns, "mora.math/atan2", native_atan2);

    // Hyperbolic
    register_native(ns, "mora.math/sinh", native_sinh);
    register_native(ns, "mora.math/cosh", native_cosh);
    register_native(ns, "mora.math/tanh", native_tanh);

    // IEEE 754 remainder
    register_native(ns, "mora.math/IEEE-remainder", native_ieee_remainder);
    register_native(ns, "mora.math/copy-sign", native_copy_sign);
    register_native(ns, "mora.math/next-after", native_next_after);
    register_native(ns, "mora.math/next-up", native_next_up);
    register_native(ns, "mora.math/next-down", native_next_down);

    // Random
    register_native(ns, "mora.math/random", native_random);
}

// --- Helpers ---

fn to_f64(v: &Value) -> Result<f64, String> {
    match v {
        Value::Int(n) => Ok(*n as f64),
        Value::Float(n) => Ok(*n),
        _ => Err(format!("expected number, got {}", v.type_name())),
    }
}

// --- Basic ---

fn native_math_abs(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("abs requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n.abs())),
        Value::Float(n) => Ok(Value::Float(n.abs())),
        _ => Err("abs requires a number".to_string()),
    }
}

fn native_signum(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("signum requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Float(if *n > 0 {
            1.0
        } else if *n < 0 {
            -1.0
        } else {
            0.0
        })),
        Value::Float(n) => Ok(Value::Float(if *n > 0.0 {
            1.0
        } else if *n < 0.0 {
            -1.0
        } else {
            0.0
        })),
        _ => Err("signum requires a number".to_string()),
    }
}

fn native_round(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("round requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(*n)),
        Value::Float(n) => Ok(Value::Int(n.round() as i64)),
        _ => Err("round requires a number".to_string()),
    }
}

fn native_floor(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("floor requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Float(*n as f64)),
        Value::Float(n) => Ok(Value::Float(n.floor())),
        _ => Err("floor requires a number".to_string()),
    }
}

fn native_ceil(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("ceil requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Float(*n as f64)),
        Value::Float(n) => Ok(Value::Float(n.ceil())),
        _ => Err("ceil requires a number".to_string()),
    }
}

fn native_ulp(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("ulp requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.next_up() - n))
}

// --- Exponent / logarithm ---

fn native_pow(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("pow requires exactly 2 arguments".to_string());
    }
    let base = to_f64(&args[0])?;
    let exp = to_f64(&args[1])?;
    Ok(Value::Float(base.powf(exp)))
}

fn native_exp(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("exp requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.exp()))
}

fn native_log(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("log requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.ln()))
}

fn native_log10(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("log10 requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.log10()))
}

fn native_log2(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("log2 requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.log2()))
}

fn native_sqrt(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("sqrt requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.sqrt()))
}

fn native_cbrt(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cbrt requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.cbrt()))
}

fn native_hypot(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("hypot requires exactly 2 arguments".to_string());
    }
    let a = to_f64(&args[0])?;
    let b = to_f64(&args[1])?;
    Ok(Value::Float(a.hypot(b)))
}

fn native_expm1(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("expm1 requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.exp_m1()))
}

fn native_log1p(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("log1p requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.ln_1p()))
}

// --- Trigonometric ---

fn native_sin(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("sin requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.sin()))
}

fn native_cos(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cos requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.cos()))
}

fn native_tan(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("tan requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.tan()))
}

fn native_asin(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("asin requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.asin()))
}

fn native_acos(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("acos requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.acos()))
}

fn native_atan(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("atan requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.atan()))
}

fn native_atan2(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("atan2 requires exactly 2 arguments".to_string());
    }
    let y = to_f64(&args[0])?;
    let x = to_f64(&args[1])?;
    Ok(Value::Float(y.atan2(x)))
}

// --- Hyperbolic ---

fn native_sinh(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("sinh requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.sinh()))
}

fn native_cosh(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cosh requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.cosh()))
}

fn native_tanh(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("tanh requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.tanh()))
}

// --- IEEE 754 ---

fn native_ieee_remainder(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("IEEE-remainder requires exactly 2 arguments".to_string());
    }
    let x = to_f64(&args[0])?;
    let y = to_f64(&args[1])?;
    Ok(Value::Float(x.rem_euclid(y)))
}

fn native_copy_sign(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("copy-sign requires exactly 2 arguments".to_string());
    }
    let x = to_f64(&args[0])?;
    let sign = to_f64(&args[1])?;
    Ok(Value::Float(x.copysign(sign)))
}

fn native_next_after(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("next-after requires exactly 2 arguments".to_string());
    }
    let x = to_f64(&args[0])?;
    let direction = to_f64(&args[1])?;
    Ok(Value::Float(if direction > x {
        x.next_up()
    } else if direction < x {
        x.next_down()
    } else {
        x
    }))
}

fn native_next_up(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("next-up requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.next_up()))
}

fn native_next_down(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("next-down requires exactly 1 argument".to_string());
    }
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.next_down()))
}

// --- Random ---

fn native_random(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(rand_f64()))
}

fn rand_f64() -> f64 {
    // Simple xorshift64* PRNG — no external dependency needed
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut state = STATE.load(Ordering::Relaxed);
    if state == 0 {
        // Seed from time
        state = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
    }
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    STATE.store(state, Ordering::Relaxed);
    (state.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
}
