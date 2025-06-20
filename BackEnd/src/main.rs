use chrono::Utc;
use mongodb::bson::{doc, DateTime, Document};
use mongodb::{sync::Client};
use serialport::SerialPort;
use std::io::{BufRead, BufReader};
use std::time::Duration;

// Newton-Raphson untuk NTC menggunakan model Beta
fn newton_raphson(resistance: f64, r0: f64, beta: f64, t0: f64) -> f64 {
    let mut t = 298.15; // tebakan awal: 25°C dalam Kelvin
    for _ in 0..10 {
        let exp = beta * (1.0 / t - 1.0 / t0);
        let f = r0 * exp.exp() - resistance;
        let df = -r0 * exp.exp() * beta / (t * t);
        t -= f / df;
    }
    t - 273.15 // konversi ke °C
}

fn parse_data(data: &str) -> Option<(f64, f64)> {
    // Format: temp,resistance\n misal: 26.5,10520.4
    let parts: Vec<&str> = data.trim().split(',').collect();
    if parts.len() >= 2 {
        let temp = parts[0].trim().parse::<f64>().ok()?;
        let res = parts[1].trim().parse::<f64>().ok()?;
        Some((temp, res))
    } else {
        None
    }
}

fn main() -> mongodb::error::Result<()> {
    // Setup MongoDB
    let client = Client::with_uri_str("mongodb://localhost:27017")?;
    let db = client.database("Alprog");
    let coll = db.collection::<Document>("Temperatur");

    // Konstanta NTC
    let r0 = 10_000.0;
    let t0 = 298.15;
    let beta = 3950.0;

    // Setup Serial Port
    let port_name = "COM4"; // ganti sesuai port Anda
    let baud_rate = 9600;
    let timeout = Duration::from_secs(2);

    let port = serialport::new(port_name, baud_rate)
        .timeout(timeout)
        .open()
        .expect("Gagal membuka serial port");

    let mut reader = BufReader::new(port);

    println!("--- MEMBACA DATA SENSOR SECARA REALTIME ---");

    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).is_ok() {
            if let Some((temperature, resistance)) = parse_data(&line) {
                let calculated_temp = newton_raphson(resistance, r0, beta, t0);
                println!(
                    "[{}] Sensor: {:.5} °C, Resistansi: {:.2} Ω, Numerik: {:.5} °C",
                    Utc::now(),
                    temperature,
                    resistance,
                    calculated_temp
                );

                let doc = doc! {
                    "Waktu": DateTime::now(),
                    "suhu": temperature,
                    "resistance": resistance,
                    "numerik": calculated_temp
                };
                coll.insert_one(doc, None)?;
            } else {
                println!("Format data salah: {}", line);
            }
        }
    }
}
