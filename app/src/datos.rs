//! Datos de Mi Garaje: modelo, cálculos y guardado en disco.
//!
//! Todo empieza vacío. El usuario carga su moto, sus tanqueos, sus servicios y
//! sus documentos, y el estado se guarda como JSON en la carpeta privada de la
//! app (Android) o en `%APPDATA%\MiGaraje` (escritorio).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static RUTA: OnceLock<PathBuf> = OnceLock::new();
static SIN_GUARDADO: AtomicBool = AtomicBool::new(false);

/// Fija dónde se guarda el JSON. Se llama una vez al arrancar.
pub fn fijar_ruta(ruta: PathBuf) {
    let _ = RUTA.set(ruta);
}
/// En modo captura no se lee ni se escribe nada en disco.
pub fn sin_guardado(si: bool) {
    SIN_GUARDADO.store(si, Ordering::Relaxed);
}

/// `%APPDATA%\MiGaraje\datos.json` en Windows.
pub fn ruta_escritorio() -> PathBuf {
    if let Some(base) = std::env::var_os("APPDATA") {
        return PathBuf::from(base).join("MiGaraje").join("datos.json");
    }
    PathBuf::from("datos.json")
}

fn ruta() -> Option<&'static PathBuf> {
    RUTA.get()
}

// ------------------------------------------------------------------- fechas
/// Días completos desde el 1 de enero de 1970 (UTC).
fn dias_epoca() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86_400) as i64)
        .unwrap_or(0)
}

/// Algoritmo civil-from-days (Howard Hinnant).
fn civil(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y } as i32, m as u32, d as u32)
}

pub fn hoy() -> (i32, u32, u32) {
    civil(dias_epoca())
}

pub fn hoy_txt() -> String {
    let (y, m, d) = hoy();
    format!("{d:02}/{m:02}/{y}")
}

/// Hora local aproximada (Colombia, UTC-5).
pub fn hora_local() -> u32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (((secs / 3_600) as i64 - 5).rem_euclid(24)) as u32
}

pub fn saludo() -> &'static str {
    match hora_local() {
        0..=11 => "Buenos días,",
        12..=18 => "Buenas tardes,",
        _ => "Buenas noches,",
    }
}

pub fn semana() -> &'static str {
    // 1970-01-01 fue jueves.
    const D: [&str; 7] = [
        "Domingo",
        "Lunes",
        "Martes",
        "Miércoles",
        "Jueves",
        "Viernes",
        "Sábado",
    ];
    D[((dias_epoca() + 4).rem_euclid(7)) as usize]
}

pub fn mes_nombre(m: u32) -> &'static str {
    const M: [&str; 12] = [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ];
    M[(m as usize).clamp(1, 12) - 1]
}

pub fn mes_corto(m: u32) -> &'static str {
    const M: [&str; 12] = [
        "ENE", "FEB", "MAR", "ABR", "MAY", "JUN", "JUL", "AGO", "SEP", "OCT", "NOV", "DIC",
    ];
    M[(m as usize).clamp(1, 12) - 1]
}

fn bisiesto(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn dias_mes(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if bisiesto(y) => 29,
        _ => 28,
    }
}

/// Fecha del día `dia` de hace `meses` meses.
pub fn fecha_hace_meses(meses: u32, dia: u32) -> String {
    let (y, m, _) = hoy();
    let total = y * 12 + m as i32 - 1 - meses as i32;
    let ny = total.div_euclid(12);
    let nm = (total.rem_euclid(12) + 1) as u32;
    let d = dia.clamp(1, dias_mes(ny, nm));
    format!("{d:02}/{nm:02}/{ny}")
}

/// Fecha del día `dia` dentro de `meses` meses (para vencimientos).
pub fn fecha_mas_meses(meses: u32, dia: u32) -> String {
    fecha_mas_meses_desde(hoy(), meses, dia)
}

/// Igual que `fecha_mas_meses` pero contando desde `base` (para poder probar
/// la caja inteligente con una fecha fija).
pub fn fecha_mas_meses_desde(base: (i32, u32, u32), meses: u32, dia: u32) -> String {
    let (y, m, _) = base;
    let total = y * 12 + m as i32 - 1 + meses as i32;
    let ny = total.div_euclid(12);
    let nm = (total.rem_euclid(12) + 1) as u32;
    let d = dia.clamp(1, dias_mes(ny, nm));
    format!("{d:02}/{nm:02}/{ny}")
}

/// Días desde 1970-01-01 de una fecha civil (inverso de `civil`).
fn dias_desde_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Fecha de dentro de `dias` días (puede ser negativa).
pub fn fecha_mas_dias(dias: i64) -> String {
    fecha_mas_dias_desde(hoy(), dias)
}

/// Igual que `fecha_mas_dias` pero contando desde `base`.
pub fn fecha_mas_dias_desde(base: (i32, u32, u32), dias: i64) -> String {
    let (y, m, d) = civil(dias_desde_civil(base.0, base.1, base.2) + dias);
    format!("{d:02}/{m:02}/{y}")
}

/// Convierte "dd/mm/aaaa" en (año, mes, día).
pub fn parse_fecha(txt: &str) -> Option<(i32, u32, u32)> {
    let partes: Vec<&str> = txt.trim().split('/').collect();
    if partes.len() != 3 {
        return None;
    }
    let d: u32 = partes[0].parse().ok()?;
    let m: u32 = partes[1].parse().ok()?;
    let y: i32 = partes[2].parse().ok()?;
    if !(1..=31).contains(&d) || !(1..=12).contains(&m) || !(1950..=2200).contains(&y) {
        return None;
    }
    if d > dias_mes(y, m) {
        return None;
    }
    Some((y, m, d))
}

fn clave(txt: &str) -> i64 {
    parse_fecha(txt)
        .map(|(y, m, d)| y as i64 * 10_000 + m as i64 * 100 + d as i64)
        .unwrap_or(0)
}

// ------------------------------------------------------------------ modelo
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Perfil {
    pub nombre: String,
    pub email: String,
    pub ciudad: String,
    pub telefono: String,
}

impl Perfil {
    pub fn iniciales(&self) -> String {
        let mut out = String::new();
        for p in self.nombre.split_whitespace().take(2) {
            if let Some(ch) = p.chars().next() {
                out.extend(ch.to_uppercase());
            }
        }
        if out.is_empty() {
            "MG".to_string()
        } else {
            out
        }
    }

    pub fn nombre_o(&self) -> String {
        if self.nombre.trim().is_empty() {
            "Tu nombre".to_string()
        } else {
            self.nombre.clone()
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Vehiculo {
    pub marca: String,
    pub modelo: String,
    pub year: i32,
    pub km: i64,
    pub placa: String,
    pub color: String,
    pub cc: i32,
    pub principal: bool,
}

impl Vehiculo {
    pub fn titulo(&self) -> String {
        match (self.marca.trim(), self.modelo.trim()) {
            ("", "") => "Mi moto".to_string(),
            ("", m) => m.to_string(),
            (m, "") => m.to_string(),
            (m, mo) => format!("{m} {mo}"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Tanqueo {
    pub vehiculo: usize,
    pub fecha: String,
    pub estacion: String,
    /// Lo que pagó el usuario, en pesos. Es el dato principal ahora.
    pub monto: f64,
    /// Galones guardados (modelo anterior). Se conserva para leer archivos
    /// viejos; en las cargas nuevas vale 0 y los galones se deducen del monto.
    pub galones: f32,
    pub precio_galon: f64,
    pub km: i64,
}

impl Tanqueo {
    /// Galones equivalentes: del monto y el precio, o los guardados si el
    /// tanqueo es del modelo anterior.
    pub fn galones(&self) -> f32 {
        if self.monto > 0.0 && self.precio_galon > 0.0 {
            (self.monto / self.precio_galon) as f32
        } else {
            self.galones
        }
    }

    /// Lo que costó el tanqueo.
    pub fn valor(&self) -> f64 {
        if self.monto > 0.0 {
            self.monto
        } else {
            self.galones as f64 * self.precio_galon
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Servicio {
    pub vehiculo: usize,
    pub nombre: String,
    pub detalle: String,
    pub cada_km: i64,
    pub ultimo_km: i64,
    pub costo: f64,
    pub ultima_fecha: String,
}

impl Servicio {
    pub fn faltan(&self, km_actual: i64) -> i64 {
        if self.cada_km <= 0 {
            return 0;
        }
        self.ultimo_km + self.cada_km - km_actual
    }

    pub fn avance(&self, km_actual: i64) -> f32 {
        if self.cada_km <= 0 {
            return 1.0;
        }
        ((km_actual - self.ultimo_km) as f32 / self.cada_km as f32).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Documento {
    pub vehiculo: usize,
    pub nombre: String,
    pub vence: String,
}

impl Documento {
    pub fn vigente(&self) -> bool {
        match parse_fecha(&self.vence) {
            Some((y, m, d)) => (y, m, d) >= hoy(),
            None => true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TipoMov {
    Tanqueo,
    Servicio,
}

pub struct Mov {
    pub tipo: TipoMov,
    pub titulo: String,
    pub detalle: String,
    pub valor: f64,
    pub fecha: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Datos {
    pub version: u32,
    pub perfil: Perfil,
    pub vehiculos: Vec<Vehiculo>,
    pub sel: usize,
    pub tanqueos: Vec<Tanqueo>,
    pub servicios: Vec<Servicio>,
    pub docs: Vec<Documento>,
    pub precio_galon: f64,
    pub notif: bool,
    pub recordatorios: bool,
}

impl Default for Datos {
    fn default() -> Self {
        Self {
            version: 1,
            perfil: Perfil::default(),
            vehiculos: Vec::new(),
            sel: 0,
            tanqueos: Vec::new(),
            servicios: Vec::new(),
            docs: Vec::new(),
            precio_galon: 13_900.0,
            notif: true,
            recordatorios: true,
        }
    }
}

impl Datos {
    pub fn veh(&self) -> Option<&Vehiculo> {
        if self.vehiculos.is_empty() {
            None
        } else {
            self.vehiculos.get(self.sel.min(self.vehiculos.len() - 1))
        }
    }

    pub fn indice(&self) -> usize {
        self.sel.min(self.vehiculos.len().saturating_sub(1))
    }

    pub fn docs_de(&self, vehiculo: usize) -> Vec<&Documento> {
        self.docs.iter().filter(|d| d.vehiculo == vehiculo).collect()
    }

    pub fn servicios_de(&self, vehiculo: usize) -> Vec<&Servicio> {
        self.servicios
            .iter()
            .filter(|s| s.vehiculo == vehiculo)
            .collect()
    }

    pub fn veh_mut(&mut self) -> Option<&mut Vehiculo> {
        let i = self.sel.min(self.vehiculos.len().saturating_sub(1));
        self.vehiculos.get_mut(i)
    }

    pub fn km_actual(&self) -> i64 {
        self.veh().map(|v| v.km).unwrap_or(0)
    }

    pub fn total_tanqueos(&self) -> f64 {
        let v = self.indice();
        self.tanqueos
            .iter()
            .filter(|t| t.vehiculo == v)
            .map(|t| t.valor())
            .sum()
    }

    pub fn galones_total(&self) -> f32 {
        let v = self.indice();
        self.tanqueos
            .iter()
            .filter(|t| t.vehiculo == v)
            .map(|t| t.galones())
            .sum()
    }

    /// Gastos de combustible + servicios hechos este mes.
    pub fn gasto_mes(&self) -> f64 {
        let (y, m, _) = hoy();
        self.gasto_de(y, m)
    }

    pub fn gasto_mes_anterior(&self) -> f64 {
        let (y, m, _) = hoy();
        let total = y * 12 + m as i32 - 1;
        let (py, pm) = (total.div_euclid(12), (total.rem_euclid(12) + 1) as u32);
        self.gasto_de(py, pm)
    }

    fn gasto_de(&self, y: i32, m: u32) -> f64 {
        let v = self.indice();
        let de_mes = |f: &str| matches!(parse_fecha(f), Some((yy, mm, _)) if yy == y && mm == m);
        let combustible: f64 = self
            .tanqueos
            .iter()
            .filter(|t| t.vehiculo == v && de_mes(&t.fecha))
            .map(|t| t.valor())
            .sum();
        let servicios: f64 = self
            .servicios
            .iter()
            .filter(|s| s.vehiculo == v && !s.ultima_fecha.is_empty() && de_mes(&s.ultima_fecha))
            .map(|s| s.costo)
            .sum();
        combustible + servicios
    }

    /// Últimos 7 meses: (etiqueta, total gastado).
    pub fn serie_meses(&self) -> Vec<(String, f64)> {
        let (y, m, _) = hoy();
        let actual = y * 12 + m as i32 - 1;
        (0..7)
            .rev()
            .map(|atras| {
                let total = actual - atras;
                let yy = total.div_euclid(12);
                let mm = (total.rem_euclid(12) + 1) as u32;
                (mes_corto(mm).to_string(), self.gasto_de(yy, mm))
            })
            .collect()
    }

    /// Últimos 7 meses: (etiqueta, kilometraje del último tanqueo del mes).
    pub fn serie_km(&self) -> Vec<(String, i64)> {
        let v = self.indice();
        let (y, m, _) = hoy();
        let actual = y * 12 + m as i32 - 1;
        (0..7)
            .rev()
            .map(|atras| {
                let total = actual - atras;
                let yy = total.div_euclid(12);
                let mm = (total.rem_euclid(12) + 1) as u32;
                let km = self
                    .tanqueos
                    .iter()
                    .filter(|t| {
                        t.vehiculo == v
                            && matches!(parse_fecha(&t.fecha), Some((ty, tm, _)) if ty == yy && tm == mm)
                    })
                    .map(|t| t.km)
                    .max()
                    .unwrap_or(0);
                (mes_corto(mm).to_string(), km)
            })
            .collect()
    }

    /// Gasto total (combustible + servicios) de los últimos 12 meses.
    pub fn gasto_12m(&self) -> f64 {
        let v = self.indice();
        let (y, m, _) = hoy();
        let piso = y * 12 + m as i32 - 1 - 11;
        let reciente = |f: &str| {
            parse_fecha(f)
                .map(|(yy, mm, _)| yy * 12 + mm as i32 - 1 >= piso)
                .unwrap_or(false)
        };
        let combustible: f64 = self
            .tanqueos
            .iter()
            .filter(|t| t.vehiculo == v && reciente(&t.fecha))
            .map(|t| t.valor())
            .sum();
        let servicios: f64 = self
            .servicios
            .iter()
            .filter(|s| s.vehiculo == v && !s.ultima_fecha.is_empty() && reciente(&s.ultima_fecha))
            .map(|s| s.costo)
            .sum();
        combustible + servicios
    }

    /// Kilómetros por galón, medidos entre el primer y el último tanqueo.
    pub fn consumo(&self) -> i64 {
        let v = self.indice();
        let mut ts: Vec<&Tanqueo> = self
            .tanqueos
            .iter()
            .filter(|t| t.vehiculo == v && t.km > 0 && t.galones() > 0.0)
            .collect();
        if ts.len() < 2 {
            return 0;
        }
        ts.sort_by_key(|t| t.km);
        let dist = ts.last().unwrap().km - ts.first().unwrap().km;
        let g: f32 = ts[1..].iter().map(|t| t.galones()).sum();
        if dist <= 0 || g <= 0.0 {
            0
        } else {
            (dist as f32 / g).round() as i64
        }
    }

    /// Cuántos kilómetros rinde cada $10.000 de gasolina.
    pub fn km_por_10mil(&self) -> i64 {
        let ck = self.costo_km();
        if ck <= 0.0 {
            0
        } else {
            (10_000.0 / ck).round() as i64
        }
    }

    pub fn costo_km(&self) -> f64 {
        let v = self.indice();
        let mut ts: Vec<&Tanqueo> = self
            .tanqueos
            .iter()
            .filter(|t| t.vehiculo == v && t.km > 0)
            .collect();
        if ts.len() < 2 {
            return 0.0;
        }
        ts.sort_by_key(|t| t.km);
        let dist = ts.last().unwrap().km - ts.first().unwrap().km;
        let gasto: f64 = self
            .tanqueos
            .iter()
            .filter(|t| t.vehiculo == v)
            .map(|t| t.valor())
            .sum();
        if dist <= 0 {
            0.0
        } else {
            gasto / dist as f64
        }
    }

    pub fn proximo_servicio(&self) -> Option<&Servicio> {
        let v = self.indice();
        self.servicios
            .iter()
            .filter(|s| s.vehiculo == v && s.cada_km > 0)
            .min_by_key(|s| s.faltan(self.km_actual()))
    }

    /// 1.0 = al día, baja cuando hay servicios vencidos.
    pub fn estado(&self) -> Option<f32> {
        let v = self.indice();
        let servicios: Vec<&Servicio> = self.servicios.iter().filter(|s| s.vehiculo == v).collect();
        if servicios.is_empty() {
            return None;
        }
        let km = self.km_actual();
        let vencidos = servicios.iter().filter(|s| s.faltan(km) <= 0).count();
        Some(1.0 - 0.5 * (vencidos as f32 / servicios.len() as f32))
    }

    /// Movimientos recientes (tanqueos y servicios) del más nuevo al más viejo.
    pub fn movimientos(&self) -> Vec<Mov> {
        let v = self.indice();
        let mut out: Vec<Mov> = self
            .tanqueos
            .iter()
            .filter(|t| t.vehiculo == v)
            .map(|t| Mov {
                tipo: TipoMov::Tanqueo,
                titulo: if t.estacion.trim().is_empty() {
                    "Tanqueo".to_string()
                } else {
                    format!("Tanqueo · {}", t.estacion)
                },
                detalle: format!("{:.1} gal · {} km", t.galones(), t.km),
                valor: t.valor(),
                fecha: t.fecha.clone(),
            })
            .chain(
                self.servicios
                    .iter()
                    .filter(|s| s.vehiculo == v && !s.ultima_fecha.is_empty())
                    .map(|s| Mov {
                        tipo: TipoMov::Servicio,
                        titulo: s.nombre.clone(),
                        detalle: "Servicio realizado".to_string(),
                        valor: s.costo,
                        fecha: s.ultima_fecha.clone(),
                    }),
            )
            .collect();
        out.sort_by_key(|m| -clave(&m.fecha));
        out
    }

    /// Datos de muestra, solo para capturas y pruebas.
    pub fn demo() -> Self {
        let mut d = Datos {
            perfil: Perfil {
                nombre: "Daniel Miranda".into(),
                email: "daniel@correo.com".into(),
                ciudad: "Bogotá D.C.".into(),
                telefono: "+57 300 000 0000".into(),
            },
            precio_galon: 13_900.0,
            ..Default::default()
        };

        d.vehiculos.push(Vehiculo {
            marca: "Yamaha".into(),
            modelo: "R15 V2".into(),
            year: 2019,
            km: 12_480,
            placa: "KVR 12C".into(),
            color: "Azul racing".into(),
            cc: 150,
            principal: true,
        });

        let kms = [8_200, 9_100, 9_950, 10_750, 11_400, 12_010, 12_480];
        let galones = [3.4, 3.0, 3.2, 3.1, 3.3, 3.0, 3.2];
        for i in 0..7 {
            let meses = (6 - i) as u32;
            let precio = 13_900.0 - (6 - i) as f64 * 120.0;
            d.tanqueos.push(Tanqueo {
                vehiculo: 0,
                fecha: fecha_hace_meses(meses, 12),
                estacion: ["Terpel · Calle 80", "Primax · Autopista Sur", "Texaco · Salitre"][i % 3]
                    .into(),
                monto: (galones[i] as f64 * precio).round(),
                galones: 0.0,
                precio_galon: precio,
                km: kms[i],
            });
        }

        d.servicios.push(Servicio {
            vehiculo: 0,
            nombre: "Cambio de aceite".into(),
            detalle: "10W-40 · cada 3.000 km".into(),
            cada_km: 3_000,
            ultimo_km: 11_000,
            costo: 85_000.0,
            ultima_fecha: fecha_hace_meses(1, 8),
        });
        d.servicios.push(Servicio {
            vehiculo: 0,
            nombre: "Kit de arrastre".into(),
            detalle: "Cadena y piñones · 12.000 km".into(),
            cada_km: 12_000,
            ultimo_km: 4_500,
            costo: 145_000.0,
            ultima_fecha: fecha_hace_meses(5, 2),
        });
        d.servicios.push(Servicio {
            vehiculo: 0,
            nombre: "Pastillas de freno".into(),
            detalle: "Delanteras y traseras".into(),
            cada_km: 8_000,
            ultimo_km: 6_000,
            costo: 120_000.0,
            ultima_fecha: String::new(),
        });

        let (y, _, _) = hoy();
        d.docs.push(Documento {
            vehiculo: 0,
            nombre: "SOAT".into(),
            vence: format!("14/03/{}", y + 1),
        });
        d.docs.push(Documento {
            vehiculo: 0,
            nombre: "Tecnomecánica".into(),
            vence: format!("22/07/{}", y + 1),
        });
        d
    }
}

// --------------------------------------------------------------- guardado
pub fn cargar() -> Datos {
    if SIN_GUARDADO.load(Ordering::Relaxed) {
        return Datos::default();
    }
    let Some(ruta) = ruta() else {
        return Datos::default();
    };
    if let Ok(bytes) = std::fs::read(ruta) {
        if let Ok(datos) = serde_json::from_slice(&bytes) {
            return datos;
        }
    }

    if let Some(backup) = ruta.with_extension("json.bak").as_path().to_path_buf().parent().map(|_| ruta.with_extension("json.bak")) {
        if let Ok(bytes) = std::fs::read(backup) {
            if let Ok(datos) = serde_json::from_slice(&bytes) {
                return datos;
            }
        }
    }
    Datos::default()
}

pub fn guardar(datos: &Datos) -> Result<(), String> {
    if SIN_GUARDADO.load(Ordering::Relaxed) {
        return Ok(());
    }
    let Some(ruta) = ruta() else {
        return Ok(());
    };
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("No pude crear la carpeta: {e}"))?;
    }
    let json = serde_json::to_vec_pretty(datos).map_err(|e| format!("No pude serializar: {e}"))?;
    let temporal = ruta.with_extension("json.tmp");

    std::fs::write(&temporal, &json).map_err(|e| format!("No pude preparar el guardado: {e}"))?;

    let backup = ruta.with_extension("json.bak");
    if ruta.exists() {
        let _ = std::fs::copy(ruta, &backup);
    }

    #[cfg(windows)]
    if ruta.exists() {
        let _ = std::fs::remove_file(ruta);
    }

    std::fs::rename(&temporal, ruta).map_err(|e| format!("No pude guardar: {e}"))
}

/// Borra el archivo de datos.
pub fn borrar() {
    if let Some(ruta) = ruta() {
        let _ = std::fs::remove_file(ruta);
    }
}

/// Serializa el estado a JSON con formato legible.
pub fn a_json(datos: &Datos) -> Result<String, String> {
    serde_json::to_string_pretty(datos).map_err(|e| format!("No pude serializar: {e}"))
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::sync::Mutex;

    static PRUEBA_RUTA: Mutex<()> = Mutex::new(());

    #[test]
    fn arranca_vacio() {
        let d = Datos::default();
        assert!(d.vehiculos.is_empty());
        assert!(d.tanqueos.is_empty());
        assert!(d.servicios.is_empty());
        assert!(d.docs.is_empty());
        assert_eq!(d.indice(), 0);
        assert!(d.veh().is_none());
        assert_eq!(d.km_actual(), 0);
    }

    #[test]
    fn guarda_y_carga() {
        let _bloqueo = PRUEBA_RUTA.lock().unwrap();
        let dir = std::env::temp_dir().join("migaraje_prueba");
        let _ = std::fs::create_dir_all(&dir);
        let ruta = dir.join("datos.json");
        let _ = std::fs::remove_file(&ruta);
        fijar_ruta(ruta.clone());

        let mut d = Datos::default();
        d.vehiculos.push(Vehiculo {
            marca: "Yamaha".into(),
            modelo: "R15 V2".into(),
            year: 2019,
            km: 1000,
            placa: "ABC12D".into(),
            color: "Azul".into(),
            cc: 150,
            principal: true,
        });
        d.tanqueos.push(Tanqueo {
            vehiculo: 0,
            fecha: hoy_txt(),
            estacion: "Terpel".into(),
            monto: 0.0,
            galones: 3.0,
            precio_galon: 16000.0,
            km: 1200,
        });
        d.servicios.push(Servicio {
            vehiculo: 0,
            nombre: "Aceite".into(),
            detalle: String::new(),
            cada_km: 3000,
            ultimo_km: 1000,
            costo: 80000.0,
            ultima_fecha: hoy_txt(),
        });
        d.docs.push(Documento {
            vehiculo: 0,
            nombre: "SOAT".into(),
            vence: fecha_mas_meses(12, 15),
        });
        guardar(&d).expect("debería guardar");

        let c = cargar();
        assert_eq!(c.vehiculos.len(), 1);
        assert_eq!(c.vehiculos[0].modelo, "R15 V2");
        assert_eq!(c.tanqueos.len(), 1);
        assert!((c.tanqueos[0].valor() - 48000.0).abs() < 0.01);
        assert!((c.tanqueos[0].galones() - 3.0).abs() < 0.001);
        assert_eq!(c.servicios.len(), 1);
        assert_eq!(c.docs.len(), 1);
        assert!(c.docs[0].vigente());
        assert_eq!(c.km_actual(), 1000);

        // y el archivo existe de verdad
        assert!(ruta.exists());
        let _ = std::fs::remove_file(&ruta);
    }

    #[test]
    fn recupera_backup_si_el_json_actual_esta_danado() {
        let _bloqueo = PRUEBA_RUTA.lock().unwrap();
        let dir = std::env::temp_dir().join("migaraje_backup_prueba");
        let _ = std::fs::create_dir_all(&dir);
        let ruta = dir.join("datos.json");
        let backup = dir.join("datos.json.bak");
        let _ = std::fs::remove_file(&ruta);
        let _ = std::fs::remove_file(&backup);
        fijar_ruta(ruta.clone());

        let mut d = Datos::default();
        d.perfil.nombre = "Ana".into();
        guardar(&d).expect("debería guardar");
        guardar(&d).expect("debería crear backup");

        std::fs::write(&ruta, b"{ datos incompletos").expect("debería dañar el archivo");
        let recuperado = cargar();
        assert_eq!(recuperado.perfil.nombre, "Ana");

        let _ = std::fs::remove_file(&ruta);
        let _ = std::fs::remove_file(&backup);
    }

    #[test]
    fn tanqueo_por_monto() {
        // Modelo nuevo: el usuario escribe la plata, no los galones.
        let t = Tanqueo {
            vehiculo: 0,
            fecha: hoy_txt(),
            estacion: "Terpel".into(),
            monto: 20_000.0,
            galones: 0.0,
            precio_galon: 13_900.0,
            km: 5_000,
        };
        assert!((t.valor() - 20_000.0).abs() < 0.01);
        // 20.000 / 13.900 ≈ 1,44 galones
        assert!((t.galones() - 1.4388).abs() < 0.01);

        // Un archivo viejo (solo galones) sigue funcionando.
        let viejo = Tanqueo {
            galones: 2.0,
            precio_galon: 13_000.0,
            ..Default::default()
        };
        assert!((viejo.valor() - 26_000.0).abs() < 0.01);
        assert!((viejo.galones() - 2.0).abs() < 0.001);
    }

    #[test]
    fn rendimiento_por_plata() {
        let d = Datos::demo();
        assert!(d.consumo() > 0);
        assert!(d.km_por_10mil() > 0);
        // Rendimiento coherente: km/$10.000 = 10.000 / costo_km.
        let esperado = (10_000.0 / d.costo_km()).round() as i64;
        assert_eq!(d.km_por_10mil(), esperado);
    }

    #[test]
    fn calculos_del_demo() {
        let d = Datos::demo();
        assert_eq!(d.vehiculos.len(), 1);
        assert!(d.consumo() > 0);
        assert!(d.galones_total() > 0.0);
        assert!(d.gasto_12m() > 0.0);
        assert_eq!(d.serie_meses().len(), 7);
        assert_eq!(d.serie_km().len(), 7);
        assert!(d.movimientos().len() >= 3);
        assert!(d.estado().is_some());
        assert!(d.costo_km() > 0.0);
        assert!(d.proximo_servicio().is_some());
    }

    #[test]
    fn fechas() {
        assert_eq!(fecha_mas_meses(0, 1), fecha_hace_meses(0, 1));
        assert!(parse_fecha("29/02/2024").is_some());
        assert!(parse_fecha("29/02/2023").is_none());
        assert!(parse_fecha("01/13/2026").is_none());
        assert!(parse_fecha("hola").is_none());
    }
}
