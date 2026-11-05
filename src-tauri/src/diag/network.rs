use core::ffi::c_void;
use core::ptr;
use std::net::{IpAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcmpAnalysisResult {
    pub target_host: String,
    pub pings_sent: usize,
    pub pings_received: usize,
    pub packet_loss_pct: f64,
    pub avg_ping_ms: f64,
    pub min_ping_ms: f64,
    pub max_ping_ms: f64,
    pub jitter_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpAnalysisResult {
    pub target_host: String,
    pub target_port: u16,
    pub packets_sent: usize,
    pub packets_received: usize,
    pub packet_loss_pct: f64,
    pub avg_rtt_ms: f64,
    pub jitter_ms: f64,
    pub min_rtt_ms: f64,
    pub max_rtt_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAnalysisResult {
    pub host: String,
    pub icmp: IcmpAnalysisResult,
    pub udp: UdpAnalysisResult,
    pub status: String,        // "optimo" | "aceptable" | "critico"
    pub status_label: String,
    pub explanation: String,
}

// -----------------------------------------------------------------------------
// Win32 ICMP FFI (iphlpapi.dll)
// -----------------------------------------------------------------------------

#[repr(C)]
struct IP_OPTION_INFORMATION {
    ttl: u8,
    tos: u8,
    flags: u8,
    options_size: u8,
    options_data: *mut c_void,
}

#[repr(C)]
struct ICMP_ECHO_REPLY {
    address: u32,
    status: u32,
    round_trip_time: u32,
    data_size: u16,
    reserved: u16,
    data: *mut c_void,
    options: IP_OPTION_INFORMATION,
}

#[link(name = "iphlpapi")]
extern "system" {
    fn IcmpCreateFile() -> *mut c_void;
    fn IcmpCloseHandle(icmp_handle: *mut c_void) -> i32;
    fn IcmpSendEcho(
        icmp_handle: *mut c_void,
        destination_address: u32,
        request_data: *const c_void,
        request_size: u16,
        request_options: *const IP_OPTION_INFORMATION,
        reply_buffer: *mut c_void,
        reply_size: u32,
        timeout: u32,
    ) -> u32;
}

pub fn run_network_analysis(host: &str) -> AppResult<NetworkAnalysisResult> {
    let target = if host.trim().is_empty() {
        "8.8.8.8"
    } else {
        host.trim()
    };

    let overall_start = Instant::now();

    // Resolve IP
    let ip_addr = resolve_host_ipv4(target)?;
    let ip_u32 = match ip_addr {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            u32::from_ne_bytes(octets)
        }
        IpAddr::V6(_) => {
            return Err(AppError::NetworkAnalysis(
                "Análisis actualmente optimizado para destinos IPv4".to_string(),
            ));
        }
    };

    // 1. ICMP Ping Analysis (20 pings)
    let icmp_result = measure_icmp_pings(target, ip_u32, 20)?;

    // 3s Timeout Guard: If all pings failed or elapsed exceeds limit without responses
    if icmp_result.pings_received == 0 && overall_start.elapsed() >= Duration::from_secs(3) {
        return Err(AppError::NetworkAnalysis(
            "Tiempo de espera agotado (3s). El host no responde a solicitudes de red.".to_string(),
        ));
    }

    // 2. UDP Jitter Analysis (50 packets @ 15.6ms tickrate 64 simulation)
    let udp_result = measure_udp_jitter(target, ip_addr, 53, 50)?;

    // Overall Status Evaluation
    let ping = icmp_result.avg_ping_ms;
    let jitter = icmp_result.jitter_ms;
    let loss = icmp_result.packet_loss_pct;

    let (status, status_label, explanation) = if loss == 0.0 && ping < 30.0 && jitter < 5.0 {
        (
            "optimo".to_string(),
            "Óptimo (<30ms Ping / <5ms Jitter)".to_string(),
            "Excelente conexión para juego competitivo: latencia ultrabaja, cero pérdida de paquetes y fluctuación mínima en la ruta de red.".to_string(),
        )
    } else if loss == 0.0 && ping <= 80.0 && jitter <= 15.0 {
        (
            "aceptable".to_string(),
            "Aceptable (<=80ms Ping / Jitter Moderado)".to_string(),
            "Conexión estable y jugable, con latencia estándar de fibra/cable sin pérdida de paquetes.".to_string(),
        )
    } else {
        (
            "critico".to_string(),
            "Crítico (>80ms Ping o Pérdida de Paquetes)".to_string(),
            "Riesgo de retraso en hitreg o micro-congelaciones: se detectó ping superior a 80ms o pérdida de paquetes en el trayecto.".to_string(),
        )
    };

    Ok(NetworkAnalysisResult {
        host: target.to_string(),
        icmp: icmp_result,
        udp: udp_result,
        status,
        status_label,
        explanation,
    })
}

fn resolve_host_ipv4(host: &str) -> AppResult<IpAddr> {
    let query = format!("{host}:53");
    let addrs = query
        .to_socket_addrs()
        .map_err(|e| AppError::NetworkAnalysis(format!("No se pudo resolver el host '{host}': {e}")))?;

    for addr in addrs {
        if addr.is_ipv4() {
            return Ok(addr.ip());
        }
    }

    Err(AppError::NetworkAnalysis(format!(
        "No se encontró dirección IPv4 para el host '{host}'"
    )))
}

fn measure_icmp_pings(host: &str, ip_u32: u32, count: usize) -> AppResult<IcmpAnalysisResult> {
    let handle = unsafe { IcmpCreateFile() };
    if handle.is_null() || handle == (-1isize as *mut c_void) {
        return Err(AppError::NetworkAnalysis(
            "No se pudo inicializar interfaz ICMP nativa (IcmpCreateFile)".to_string(),
        ));
    }

    struct IcmpGuard(*mut c_void);
    impl Drop for IcmpGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = IcmpCloseHandle(self.0);
            }
        }
    }
    let _guard = IcmpGuard(handle);

    let send_data = [0xAAu8; 32];
    let reply_size = (std::mem::size_of::<ICMP_ECHO_REPLY>() + 32 + 8) as u32;
    let mut reply_buf = vec![0u8; reply_size as usize];

    let mut rtts: Vec<f64> = Vec::with_capacity(count);
    let mut received = 0usize;

    for _ in 0..count {
        let opt = IP_OPTION_INFORMATION {
            ttl: 64,
            tos: 0,
            flags: 0,
            options_size: 0,
            options_data: ptr::null_mut(),
        };

        let replies = unsafe {
            IcmpSendEcho(
                handle,
                ip_u32,
                send_data.as_ptr() as *const c_void,
                send_data.len() as u16,
                &opt,
                reply_buf.as_mut_ptr() as *mut c_void,
                reply_size,
                1000, // 1s timeout per packet
            )
        };

        if replies > 0 {
            let reply = unsafe { &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY) };
            if reply.status == 0 {
                received += 1;
                rtts.push(reply.round_trip_time as f64);
            }
        }

        // Short pause between pings
        std::thread::sleep(Duration::from_millis(40));
    }

    let loss_pct = if count > 0 {
        ((count - received) as f64 / count as f64) * 100.0
    } else {
        100.0
    };

    if rtts.is_empty() {
        return Ok(IcmpAnalysisResult {
            target_host: host.to_string(),
            pings_sent: count,
            pings_received: 0,
            packet_loss_pct: 100.0,
            avg_ping_ms: 0.0,
            min_ping_ms: 0.0,
            max_ping_ms: 0.0,
            jitter_ms: 0.0,
        });
    }

    let sum: f64 = rtts.iter().sum();
    let avg = sum / (rtts.len() as f64);
    let min = rtts.iter().copied().fold(f64::INFINITY, f64::min);
    let max = rtts.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let variance: f64 = rtts
        .iter()
        .map(|rtt| {
            let diff = rtt - avg;
            diff * diff
        })
        .sum::<f64>()
        / (rtts.len() as f64);
    let jitter = variance.sqrt();

    Ok(IcmpAnalysisResult {
        target_host: host.to_string(),
        pings_sent: count,
        pings_received: received,
        packet_loss_pct: (loss_pct * 10.0).round() / 10.0,
        avg_ping_ms: (avg * 10.0).round() / 10.0,
        min_ping_ms: (min * 10.0).round() / 10.0,
        max_ping_ms: (max * 10.0).round() / 10.0,
        jitter_ms: (jitter * 100.0).round() / 100.0,
    })
}

fn measure_udp_jitter(
    host: &str,
    ip_addr: IpAddr,
    target_port: u16,
    packet_count: usize,
) -> AppResult<UdpAnalysisResult> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| AppError::NetworkAnalysis(format!("No se pudo enlazar socket UDP: {e}")))?;

    socket
        .set_read_timeout(Some(Duration::from_millis(150)))
        .map_err(|e| AppError::NetworkAnalysis(format!("Fallo al configurar timeout UDP: {e}")))?;

    let target_dest = format!("{ip_addr}:{target_port}");

    // Standard minimal DNS header probe (12 bytes)
    let mut probe_packet = [
        0x13, 0x37, // Transaction ID
        0x01, 0x00, // Standard query
        0x00, 0x01, // QDCOUNT = 1
        0x00, 0x00, 0x00, 0x00,
        0x00,       // Root label
        0x00, 0x01, // Type A
        0x00, 0x01, // Class IN
    ];

    let mut rtts: Vec<f64> = Vec::with_capacity(packet_count);
    let mut received = 0usize;
    let mut recv_buf = [0u8; 512];

    let target_interval = Duration::from_micros(15625); // ~15.6 ms tickrate 64

    for seq in 0..packet_count {
        let tick_start = Instant::now();

        // Unique sequence ID
        probe_packet[0] = ((seq >> 8) & 0xFF) as u8;
        probe_packet[1] = (seq & 0xFF) as u8;

        if socket.send_to(&probe_packet, &target_dest).is_ok() {
            let send_time = Instant::now();
            if let Ok((len, _from)) = socket.recv_from(&mut recv_buf) {
                if len >= 2 && recv_buf[0] == probe_packet[0] && recv_buf[1] == probe_packet[1] {
                    let rtt_ms = send_time.elapsed().as_secs_f64() * 1000.0;
                    rtts.push(rtt_ms);
                    received += 1;
                }
            }
        }

        // Wait remainder of 15.6 ms tick
        let elapsed = tick_start.elapsed();
        if elapsed < target_interval {
            std::thread::sleep(target_interval - elapsed);
        }
    }

    let loss_pct = if packet_count > 0 {
        ((packet_count - received) as f64 / packet_count as f64) * 100.0
    } else {
        100.0
    };

    if rtts.is_empty() {
        return Ok(UdpAnalysisResult {
            target_host: host.to_string(),
            target_port,
            packets_sent: packet_count,
            packets_received: 0,
            packet_loss_pct: 100.0,
            avg_rtt_ms: 0.0,
            jitter_ms: 0.0,
            min_rtt_ms: 0.0,
            max_rtt_ms: 0.0,
        });
    }

    let sum: f64 = rtts.iter().sum();
    let avg = sum / (rtts.len() as f64);
    let min = rtts.iter().copied().fold(f64::INFINITY, f64::min);
    let max = rtts.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let variance: f64 = rtts
        .iter()
        .map(|rtt| {
            let diff = rtt - avg;
            diff * diff
        })
        .sum::<f64>()
        / (rtts.len() as f64);
    let jitter = variance.sqrt();

    Ok(UdpAnalysisResult {
        target_host: host.to_string(),
        target_port,
        packets_sent: packet_count,
        packets_received: received,
        packet_loss_pct: (loss_pct * 10.0).round() / 10.0,
        avg_rtt_ms: (avg * 10.0).round() / 10.0,
        jitter_ms: (jitter * 100.0).round() / 100.0,
        min_rtt_ms: (min * 10.0).round() / 10.0,
        max_rtt_ms: (max * 10.0).round() / 10.0,
    })
}
