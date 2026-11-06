use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleSummary {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub std_dev: f64,
    pub p1: f64,       // 1st percentile (lower tail)
    pub p0_1: f64,     // 0.1th percentile (lower tail)
    pub p99: f64,      // 99th percentile (upper tail / frame time 1% low)
    pub p99_9: f64,    // 99.9th percentile (upper tail / frame time 0.1% low)
    pub fps_1pct_low: Option<f64>,  // 1000.0 / p99 (for frame time in ms)
    pub fps_01pct_low: Option<f64>, // 1000.0 / p99_9 (for frame time in ms)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTestResult {
    pub t_stat: f64,
    pub degrees_of_freedom: f64,
    pub p_value: f64,
    pub is_significant: bool,
    pub cohens_d: Option<f64>,
    pub mann_whitney_u: Option<f64>,
    pub mann_whitney_p: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLdatInput {
    pub mean_a: f64,
    pub std_dev_a: f64,
    pub count_a: Option<usize>,
    pub mean_b: f64,
    pub std_dev_b: f64,
    pub count_b: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLdatResult {
    pub t_test: TTestResult,
    pub delta_pct: f64,
    pub verdict: String, // "mejoro" | "placebo" | "empeoro"
    pub verdict_label: String,
    pub explanation: String,
}

/// Computes the arithmetic mean of a slice.
pub fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().copied().sum();
    Some(sum / values.len() as f64)
}

/// Computes sample variance (with Bessel's correction, N - 1 degrees of freedom).
pub fn variance(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let m = mean(values)?;
    let sum_sq_diff: f64 = values.iter().map(|&x| (x - m).powi(2)).sum();
    Some(sum_sq_diff / (values.len() - 1) as f64)
}

/// Computes sample standard deviation.
pub fn std_dev(values: &[f64]) -> Option<f64> {
    variance(values).map(|v| v.sqrt())
}

/// Computes the median of a slice.
pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        Some(sorted[n / 2])
    } else {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    }
}

/// Computes a percentile using linear interpolation between closest ranks.
/// `p` accepts either [0.0, 1.0] or [0.0, 100.0].
pub fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let fraction = if p > 1.0 { p / 100.0 } else { p }.clamp(0.0, 1.0);
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if sorted.len() == 1 {
        return Some(sorted[0]);
    }

    let index = fraction * (sorted.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let weight = index - lower as f64;

    if lower == upper {
        Some(sorted[lower])
    } else {
        Some(sorted[lower] * (1.0 - weight) + sorted[upper] * weight)
    }
}

/// Computes the 1% lower-tail percentile.
pub fn percentile_1pct(values: &[f64]) -> Option<f64> {
    percentile(values, 0.01)
}

/// Computes the 0.1% lower-tail percentile.
pub fn percentile_01pct(values: &[f64]) -> Option<f64> {
    percentile(values, 0.001)
}

/// Computes comprehensive statistical distribution summary.
pub fn compute_summary(values: &[f64]) -> Option<SampleSummary> {
    if values.is_empty() {
        return None;
    }

    let count = values.len();
    let m = mean(values)?;
    let med = median(values)?;
    let s = if count >= 2 { std_dev(values)? } else { 0.0 };

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = sorted[0];
    let max = sorted[count - 1];

    let p1 = percentile(&sorted, 0.01).unwrap_or(min);
    let p0_1 = percentile(&sorted, 0.001).unwrap_or(min);
    let p99 = percentile(&sorted, 0.99).unwrap_or(max);
    let p99_9 = percentile(&sorted, 0.999).unwrap_or(max);

    let fps_1pct_low = if p99 > 0.0 { Some((1000.0 / p99 * 10.0).round() / 10.0) } else { None };
    let fps_01pct_low = if p99_9 > 0.0 { Some((1000.0 / p99_9 * 10.0).round() / 10.0) } else { None };

    Some(SampleSummary {
        count,
        mean: (m * 1000.0).round() / 1000.0,
        median: (med * 1000.0).round() / 1000.0,
        min: (min * 1000.0).round() / 1000.0,
        max: (max * 1000.0).round() / 1000.0,
        std_dev: (s * 1000.0).round() / 1000.0,
        p1: (p1 * 1000.0).round() / 1000.0,
        p0_1: (p0_1 * 1000.0).round() / 1000.0,
        p99: (p99 * 1000.0).round() / 1000.0,
        p99_9: (p99_9 * 1000.0).round() / 1000.0,
        fps_1pct_low,
        fps_01pct_low,
    })
}

/// Welch's two-sample t-test for unequal variances and sample sizes.
/// Returns t-statistic, degrees of freedom, two-tailed p-value, significance, Cohen's d and Mann-Whitney U.
pub fn welch_t_test(sample_a: &[f64], sample_b: &[f64]) -> Option<TTestResult> {
    if sample_a.len() < 2 || sample_b.len() < 2 {
        return None;
    }
    let m1 = mean(sample_a)?;
    let m2 = mean(sample_b)?;
    let s1 = std_dev(sample_a)?;
    let s2 = std_dev(sample_b)?;
    let mut result = welch_t_test_from_stats(m1, s1, sample_a.len(), m2, s2, sample_b.len())?;

    result.cohens_d = cohens_d(sample_a, sample_b);
    if let Some((u, p)) = mann_whitney_u(sample_a, sample_b) {
        result.mann_whitney_u = Some(u);
        result.mann_whitney_p = Some(p);
    }
    Some(result)
}

/// Computes Welch's t-test directly from summary statistics: mean, standard deviation, and sample count.
pub fn welch_t_test_from_stats(
    m1: f64,
    s1: f64,
    n1: usize,
    m2: f64,
    s2: f64,
    n2: usize,
) -> Option<TTestResult> {
    let n1_f = n1 as f64;
    let n2_f = n2 as f64;

    if n1_f < 2.0 || n2_f < 2.0 {
        return None;
    }

    let v1 = s1 * s1;
    let v2 = s2 * s2;

    let se1 = v1 / n1_f;
    let se2 = v2 / n2_f;
    let se_sum = se1 + se2;

    if se_sum <= 1e-15 {
        // If both samples have zero variance and identical means
        if (m1 - m2).abs() < 1e-12 {
            return Some(TTestResult {
                t_stat: 0.0,
                degrees_of_freedom: (n1_f + n2_f - 2.0).max(1.0),
                p_value: 1.0,
                is_significant: false,
                cohens_d: Some(0.0),
                mann_whitney_u: None,
                mann_whitney_p: None,
            });
        } else {
            return Some(TTestResult {
                t_stat: if m1 > m2 { 100.0 } else { -100.0 },
                degrees_of_freedom: (n1_f + n2_f - 2.0).max(1.0),
                p_value: 0.0,
                is_significant: true,
                cohens_d: None,
                mann_whitney_u: None,
                mann_whitney_p: None,
            });
        }
    }

    let t_stat = (m1 - m2) / se_sum.sqrt();

    // Welch–Satterthwaite degrees of freedom
    let num_df = se_sum * se_sum;
    let denom_df = (se1 * se1) / (n1_f - 1.0) + (se2 * se2) / (n2_f - 1.0);
    let df = if denom_df > 0.0 {
        (num_df / denom_df).max(1.0)
    } else {
        (n1_f + n2_f - 2.0).max(1.0)
    };

    let p_value = student_t_p_value(t_stat.abs(), df);
    let is_significant = p_value < 0.05;

    Some(TTestResult {
        t_stat: (t_stat * 1000.0).round() / 1000.0,
        degrees_of_freedom: (df * 10.0).round() / 10.0,
        p_value: (p_value * 10_000.0).round() / 10_000.0,
        is_significant,
        cohens_d: None,
        mann_whitney_u: None,
        mann_whitney_p: None,
    })
}

/// Computes Cohen's d effect size between sample A and sample B.
/// d = (mean_a - mean_b) / s_pooled
pub fn cohens_d(sample_a: &[f64], sample_b: &[f64]) -> Option<f64> {
    if sample_a.len() < 2 || sample_b.len() < 2 {
        return None;
    }
    let m1 = mean(sample_a)?;
    let m2 = mean(sample_b)?;
    let n1 = sample_a.len() as f64;
    let n2 = sample_b.len() as f64;
    let v1 = variance(sample_a)?;
    let v2 = variance(sample_b)?;

    let pooled_var = ((n1 - 1.0) * v1 + (n2 - 1.0) * v2) / (n1 + n2 - 2.0);
    if pooled_var <= 1e-15 {
        return Some(0.0);
    }
    let d = (m1 - m2) / pooled_var.sqrt();
    Some((d * 1000.0).round() / 1000.0)
}

/// Computes the Mann-Whitney U test (Wilcoxon rank-sum test) for two independent samples.
/// Returns (U_statistic, two-tailed p_value).
pub fn mann_whitney_u(sample_a: &[f64], sample_b: &[f64]) -> Option<(f64, f64)> {
    let n1 = sample_a.len();
    let n2 = sample_b.len();
    if n1 < 2 || n2 < 2 {
        return None;
    }

    let mut combined: Vec<(f64, usize)> = Vec::with_capacity(n1 + n2);
    for &x in sample_a {
        combined.push((x, 0));
    }
    for &y in sample_b {
        combined.push((y, 1));
    }
    combined.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let total = combined.len();
    let mut ranks = vec![0.0; total];
    let mut i = 0;
    while i < total {
        let mut j = i;
        while j < total && (combined[j].0 - combined[i].0).abs() < 1e-12 {
            j += 1;
        }
        let avg_rank = (i + 1 + j) as f64 / 2.0;
        for k in i..j {
            ranks[k] = avg_rank;
        }
        i = j;
    }

    let mut r1 = 0.0;
    for k in 0..total {
        if combined[k].1 == 0 {
            r1 += ranks[k];
        }
    }

    let n1_f = n1 as f64;
    let n2_f = n2 as f64;
    let u1 = r1 - (n1_f * (n1_f + 1.0)) / 2.0;
    let u2 = (n1_f * n2_f) - u1;
    let u = u1.min(u2);

    let mean_u = (n1_f * n2_f) / 2.0;
    let var_u = (n1_f * n2_f * (n1_f + n2_f + 1.0)) / 12.0;
    let sigma_u = var_u.sqrt();

    let p_value = if sigma_u > 0.0 {
        let z = ((u - mean_u).abs() - 0.5).max(0.0) / sigma_u;
        erfc(z / std::f64::consts::SQRT_2).clamp(0.0, 1.0)
    } else {
        1.0
    };

    Some(((u * 10.0).round() / 10.0, (p_value * 10_000.0).round() / 10_000.0))
}

/// Complementary error function erfc(x) via Chebyshev rational approximation.
fn erfc(x: f64) -> f64 {
    if x < 0.0 {
        return 2.0 - erfc(-x);
    }
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let poly = -z * z - 1.26551223
        + t * (1.00002368
            + t * (0.37409196
                + t * (0.09678418
                    + t * (-0.18628806
                        + t * (0.27886807
                            + t * (-1.13520398
                                + t * (1.48851587
                                    + t * (-0.82215223 + t * 0.17087277))))))));
    (t * poly.exp()).clamp(0.0, 2.0)
}

/// Computes the two-tailed p-value for Student's t distribution with `df` degrees of freedom.
/// Uses the regularized incomplete beta function: p = I_{df / (df + t^2)}(df / 2, 1 / 2).
pub fn student_t_p_value(t_abs: f64, df: f64) -> f64 {
    if df <= 0.0 {
        return 1.0;
    }
    if t_abs <= 0.0 {
        return 1.0;
    }

    let x = df / (df + t_abs * t_abs);
    inc_beta(df / 2.0, 0.5, x).clamp(0.0, 1.0)
}

/// Natural log of the gamma function using Lanczos approximation (g = 5, n = 6).
fn ln_gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let coef = [
        76.18009172947146,
        -86.50532032941677,
        24.01409824083091,
        -1.231739572450155,
        0.1208650973866179e-2,
        -0.5395239384953e-5,
    ];

    let mut y = x;
    let mut tmp = x + 5.5;
    tmp -= (x + 0.5) * tmp.ln();
    let mut ser = 1.000000000190015;
    for c in &coef {
        y += 1.0;
        ser += c / y;
    }
    -tmp + (2.5066282746310005 * ser / x).ln()
}

/// Continued fraction evaluation for incomplete beta function (Lentz's method).
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    const MAXIT: usize = 120;
    const EPS: f64 = 1e-12;
    const FPMIN: f64 = 1e-30;

    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=MAXIT {
        let m_f = m as f64;
        let m2 = 2.0 * m_f;

        // Even step
        let aa = m_f * (b - m_f) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;

        // Odd step
        let aa = -(a + m_f) * (qab + m_f) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;

        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// Regularized incomplete beta function I_x(a, b).
fn inc_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    let log_beta = ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b);
    let bt = (a * x.ln() + b * (1.0 - x).ln() - log_beta).exp();

    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}
