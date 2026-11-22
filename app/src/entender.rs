//! Caja inteligente: interpreta frases en español **dentro del teléfono**, sin
//! nube ni servicios externos.
//!
//! No usa expresiones regulares ni modelos: normaliza el texto (minúsculas y
//! sin tildes) y lo analiza con reglas sencillas y legibles. Reconoce lo más
//! común del día a día:
//!
//! * documentos que vencen («el SOAT vence el 15 de marzo de 2027»),
//! * mantenimientos hechos («le hice el cambio de aceite a los 15.000 km»),
//! * tanqueos («tanqueé 50.000 a 13.900 el galón»).
//!
//! La fecha también se entiende en lenguaje natural: `15/03/2027`, `15-3-27`,
//! `15 de marzo`, `marzo de 2027`, `en 6 meses`, `el año entrante`, `hoy`…

use crate::datos;

/// Lo que la caja entendió a partir del texto.
#[derive(Clone, Debug, PartialEq)]
pub enum Idea {
    Documento {
        nombre: String,
        vence: Option<String>,
    },
    Servicio {
        nombre: String,
        cada_km: Option<i64>,
        costo: Option<f64>,
        ultimo_km: Option<i64>,
        fecha: Option<String>,
    },
    Tanqueo {
        monto: Option<f64>,
        precio: Option<f64>,
        km: Option<i64>,
        estacion: String,
    },
    /// No se reconoció nada guardable.
    Nada,
}

// --------------------------------------------------------------- palabras clave
/// Palabras que identifican un documento sin ambigüedad.
const DOC_FUERTE: &[&str] = &[
    "soat",
    "tecnomecanica",
    "tecnomecanico",
    "revision tecnica",
    "matricula",
    "tarjeta de propiedad",
    "impuesto",
    "licencia",
    "seguro",
    "poliza",
];

/// Palabras suaves: solas no bastan si también hay un servicio o un tanqueo.
const DOC_SUAVE: &[&str] = &["documento", "vence", "vencimiento", "vencio"];

const SERVICIOS: &[&str] = &[
    "mantenimiento",
    "aceite",
    "pastilla",
    "freno",
    "llanta",
    "filtro",
    "bujia",
    "sincronizacion",
    "cadena",
    "kit de arrastre",
    "revision",
    "cambie",
    "cambio",
    "ajuste",
    "valvula",
    "bateria",
    "suspension",
    "limpieza",
    "engrase",
];

const TANQUEOS: &[&str] = &[
    "tanquee",
    "tanquear",
    "tanque",
    "gasolina",
    "combustible",
    "llene",
    "cargue",
    "galon",
    "galones",
    "nafta",
    "extra",
    "corriente",
];

// ------------------------------------------------------------------ limpieza
/// Pasa el texto a minúsculas, quita tildes y deja solo lo que nos interesa
/// (letras, dígitos y separadores de números y fechas).
pub fn normalizar(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut espacio = true;
    for ch in s.chars() {
        let c = match ch {
            'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            'ñ' | 'Ñ' => 'n',
            otro => otro.to_ascii_lowercase(),
        };
        let valido =
            c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | ',' | '$' | '-' | '%');
        if valido {
            out.push(c);
            espacio = false;
        } else if !espacio {
            out.push(' ');
            espacio = true;
        }
    }
    out.trim().to_string()
}

fn contiene(texto: &str, palabras: &[&str]) -> bool {
    palabras.iter().any(|p| texto.contains(p))
}

fn tokens(texto: &str) -> Vec<String> {
    texto.split_whitespace().map(|s| s.to_string()).collect()
}

// ------------------------------------------------------------------- números
/// Convierte un token numérico colombiano a `f64`.
///
/// Entiende `50.000` (cincuenta mil), `13.900`, `50,5` y `15000`. El punto o la
/// coma con exactamente tres dígitos detrás se toman como separador de miles.
pub fn a_numero(tok: &str) -> Option<f64> {
    let limpio: String = tok
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    if limpio.is_empty() {
        return None;
    }
    let punto = limpio.rfind('.');
    let coma = limpio.rfind(',');
    let normal = match (punto, coma) {
        (Some(p), Some(c)) => {
            // El último separador es el decimal; los demás son de miles.
            if p > c {
                let ent: String = limpio[..p].chars().filter(|x| x.is_ascii_digit()).collect();
                format!("{ent}.{}", &limpio[p + 1..])
            } else {
                let ent: String = limpio[..c].chars().filter(|x| x.is_ascii_digit()).collect();
                format!("{ent}.{}", &limpio[c + 1..])
            }
        }
        (Some(p), None) => {
            if limpio[p + 1..].chars().count() == 3 {
                limpio.chars().filter(|x| x.is_ascii_digit()).collect()
            } else {
                let ent: String = limpio[..p].chars().filter(|x| x.is_ascii_digit()).collect();
                format!("{ent}.{}", &limpio[p + 1..])
            }
        }
        (None, Some(c)) => {
            if limpio[c + 1..].chars().count() == 3 {
                limpio.chars().filter(|x| x.is_ascii_digit()).collect()
            } else {
                let ent: String = limpio[..c].chars().filter(|x| x.is_ascii_digit()).collect();
                format!("{ent}.{}", &limpio[c + 1..])
            }
        }
        (None, None) => limpio,
    };
    normal.parse().ok()
}

/// Valor de un token si es un número (y no una fecha). `50k` → 50.000.
fn valor(t: &str) -> Option<f64> {
    if t.contains('/') {
        return None;
    }
    if t.matches('.').count() >= 2 {
        return None; // p. ej. 15.03.2027
    }
    let mut v = a_numero(t)?;
    if t.ends_with('k') || t.ends_with('K') {
        v *= 1000.0;
    }
    Some(v)
}

#[derive(Default)]
struct Numeros {
    km: Option<i64>,
    precio: Option<f64>,
    montos: Vec<f64>,
}

/// Reparte los números de la frase según su papel: kilometraje, precio por
/// galón o monto suelto.
fn clasificar(toks: &[String]) -> Numeros {
    let mut out = Numeros::default();
    for i in 0..toks.len() {
        let Some(v) = valor(&toks[i]) else {
            continue;
        };
        let prev = if i > 0 { toks[i - 1].as_str() } else { "" };
        let next = if i + 1 < toks.len() { toks[i + 1].as_str() } else { "" };
        let next2 = if i + 2 < toks.len() { toks[i + 2].as_str() } else { "" };

        if next.starts_with("km") || next.starts_with("kilometro") || prev.starts_with("kilometraje") {
            if out.km.is_none() {
                out.km = Some(v as i64);
            }
            continue;
        }
        let es_precio = next.starts_with("galon")
            || (matches!(next, "el" | "la" | "por" | "cada") && next2.starts_with("galon"))
            || prev.starts_with("galon")
            || prev == "precio"
            || (prev == "a" && next.starts_with("galon"));
        if es_precio {
            if out.precio.is_none() {
                out.precio = Some(v);
            }
            continue;
        }
        let v2 = if next == "mil" || next == "k" { v * 1000.0 } else { v };
        out.montos.push(v2);
    }
    out
}

// -------------------------------------------------------------------- fechas
fn fmt3(y: i32, m: u32, d: u32) -> String {
    format!("{d:02}/{m:02}/{y}")
}

fn mes_de(t: &str) -> Option<u32> {
    let t = t.trim_end_matches('.');
    Some(match t {
        "enero" | "ene" => 1,
        "febrero" | "feb" => 2,
        "marzo" | "mar" => 3,
        "abril" | "abr" => 4,
        "mayo" | "may" => 5,
        "junio" | "jun" => 6,
        "julio" | "jul" => 7,
        "agosto" | "ago" => 8,
        "septiembre" | "setiembre" | "sep" | "sept" => 9,
        "octubre" | "oct" => 10,
        "noviembre" | "nov" => 11,
        "diciembre" | "dic" => 12,
        _ => return None,
    })
}

/// Fecha escrita con números: `15/03/2027`, `15-3-2027`, `15.03.2027` o `15/03`.
fn fecha_numerica(tok: &str, hoy: (i32, u32, u32)) -> Option<String> {
    let s: String = tok
        .chars()
        .map(|c| if c == '-' || c == '.' { '/' } else { c })
        .collect();
    let partes: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    match partes.len() {
        3 => {
            let mut y: i32 = partes[2].parse().ok()?;
            if y < 100 {
                y += if y < 50 { 2000 } else { 1900 };
            }
            let t = format!("{}/{}/{}", partes[0], partes[1], y);
            datos::parse_fecha(&t).map(|(y, m, d)| fmt3(y, m, d))
        }
        2 => {
            let d: u32 = partes[0].parse().ok()?;
            let m: u32 = partes[1].parse().ok()?;
            let mut y = hoy.0;
            if (m, d) < (hoy.1, hoy.2) {
                y += 1;
            }
            datos::parse_fecha(&format!("{d}/{m}/{y}")).map(|(y, m, d)| fmt3(y, m, d))
        }
        _ => None,
    }
}

/// Busca una fecha en la frase. Devuelve `dd/mm/aaaa`.
pub fn detectar_fecha(texto: &str, hoy: (i32, u32, u32)) -> Option<String> {
    let toks = tokens(texto);

    // 1. fecha numérica
    for t in &toks {
        if let Some(f) = fecha_numerica(t, hoy) {
            return Some(f);
        }
    }

    // 2. expresiones relativas
    if texto.contains("pasado manana") {
        return Some(datos::fecha_mas_dias_desde(hoy, 2));
    }
    if texto.contains("manana") {
        return Some(datos::fecha_mas_dias_desde(hoy, 1));
    }
    if texto.contains("hoy") {
        return Some(fmt3(hoy.0, hoy.1, hoy.2));
    }
    for i in 0..toks.len() {
        if toks[i] == "en" || toks[i] == "dentro" {
            let mut j = i + 1;
            if j < toks.len() && toks[j] == "de" {
                j += 1;
            }
            if let Some(n) = toks.get(j).and_then(|t| t.parse::<u32>().ok()) {
                let u = toks.get(j + 1).map(|s| s.as_str()).unwrap_or("");
                if u.starts_with("mes") {
                    return Some(datos::fecha_mas_meses_desde(hoy, n, hoy.2));
                }
                if u.starts_with("dia") {
                    return Some(datos::fecha_mas_dias_desde(hoy, n as i64));
                }
                if u.starts_with("semana") {
                    return Some(datos::fecha_mas_dias_desde(hoy, n as i64 * 7));
                }
                if u.starts_with("ano") {
                    return Some(datos::fecha_mas_meses_desde(hoy, n * 12, hoy.2));
                }
            }
        }
    }
    if texto.contains("proximo mes") || texto.contains("mes entrante") || texto.contains("mes que viene")
    {
        return Some(datos::fecha_mas_meses_desde(hoy, 1, hoy.2));
    }
    if texto.contains("proximo ano")
        || texto.contains("ano entrante")
        || texto.contains("ano que viene")
        || texto.contains("otro ano")
    {
        return Some(datos::fecha_mas_meses_desde(hoy, 12, hoy.2));
    }

    // 3. mes con nombre («15 de marzo», «marzo de 2027», «en marzo»)
    for (i, t) in toks.iter().enumerate() {
        let Some(m) = mes_de(t) else { continue };
        let mut dia: Option<u32> = None;
        if i > 0 {
            let p = toks[i - 1].as_str();
            if p == "de" {
                if i >= 2 {
                    dia = toks[i - 2].parse::<u32>().ok().filter(|d| (1..=31).contains(d));
                }
            } else {
                dia = p.parse::<u32>().ok().filter(|d| (1..=31).contains(d));
            }
        }
        let mut anio: Option<i32> = None;
        for j in (i + 1)..toks.len().min(i + 4) {
            if let Ok(y) = toks[j].parse::<i32>() {
                if (1950..=2200).contains(&y) {
                    anio = Some(y);
                    break;
                }
            }
        }
        let d = dia.unwrap_or(1);
        let y = anio.unwrap_or_else(|| {
            if (m, d) < (hoy.1, hoy.2) {
                hoy.0 + 1
            } else {
                hoy.0
            }
        });
        if let Some(f) = datos::parse_fecha(&format!("{d}/{m}/{y}")) {
            return Some(fmt3(f.0, f.1, f.2));
        }
    }
    None
}

/// Cada cuántos km se repite un servicio («cada 3.000 km»).
fn detectar_cada(toks: &[String]) -> Option<i64> {
    for i in 0..toks.len() {
        if toks[i] == "cada" {
            if let Some(v) = toks.get(i + 1).and_then(|t| valor(t)) {
                let v = if toks.get(i + 2).map(|s| s.as_str()) == Some("mil") {
                    v * 1000.0
                } else {
                    v
                };
                return Some(v as i64);
            }
        }
    }
    None
}

// ------------------------------------------------------------------- nombres
fn nombre_documento(texto: &str) -> String {
    let mapa: &[(&str, &str)] = &[
        ("soat", "SOAT"),
        ("tecnomecanica", "Tecnomecánica"),
        ("tecnomecanico", "Tecnomecánica"),
        ("revision tecnica", "Revisión técnico-mecánica"),
        ("licencia", "Licencia de conducción"),
        ("impuesto", "Impuestos"),
        ("seguro", "Seguro"),
        ("poliza", "Póliza"),
        ("matricula", "Matrícula"),
        ("tarjeta de propiedad", "Tarjeta de propiedad"),
    ];
    for (clave, nombre) in mapa {
        if texto.contains(clave) {
            return (*nombre).to_string();
        }
    }
    "Documento".to_string()
}

fn nombre_servicio(texto: &str) -> String {
    let mapa: &[(&str, &str)] = &[
        ("aceite", "Cambio de aceite"),
        ("pastilla", "Pastillas de freno"),
        ("freno", "Frenos"),
        ("llanta", "Llantas"),
        ("filtro", "Filtro"),
        ("bujia", "Bujías"),
        ("sincronizacion", "Sincronización"),
        ("kit de arrastre", "Kit de arrastre"),
        ("cadena", "Cadena y piñones"),
        ("bateria", "Batería"),
        ("valvula", "Ajuste de válvulas"),
        ("suspension", "Suspensión"),
        ("engrase", "Engrase general"),
        ("limpieza", "Limpieza"),
    ];
    for (clave, nombre) in mapa {
        if texto.contains(clave) {
            return (*nombre).to_string();
        }
    }
    "Mantenimiento".to_string()
}

// -------------------------------------------------------------- interpretar
/// Interpreta la frase y decide qué dato del garaje representa.
pub fn interpretar(texto: &str, hoy: (i32, u32, u32)) -> Idea {
    let norm = normalizar(texto);
    if norm.is_empty() {
        return Idea::Nada;
    }
    let fecha = detectar_fecha(&norm, hoy);
    let toks = tokens(&norm);
    let nums = clasificar(&toks);
    let monto = nums.montos.iter().cloned().fold(None, |a, b| {
        Some(a.map_or(b, |x: f64| x.max(b)))
    });

    let fuerte = contiene(&norm, DOC_FUERTE);
    let suave = contiene(&norm, DOC_SUAVE);
    let serv = contiene(&norm, SERVICIOS);
    let tanq = contiene(&norm, TANQUEOS);

    if fuerte {
        return Idea::Documento {
            nombre: nombre_documento(&norm),
            vence: fecha,
        };
    }
    if tanq {
        return Idea::Tanqueo {
            monto,
            precio: nums.precio,
            km: nums.km,
            estacion: String::new(),
        };
    }
    if serv {
        return Idea::Servicio {
            nombre: nombre_servicio(&norm),
            cada_km: detectar_cada(&toks),
            costo: monto,
            ultimo_km: nums.km,
            fecha,
        };
    }
    if suave {
        return Idea::Documento {
            nombre: nombre_documento(&norm),
            vence: fecha,
        };
    }
    Idea::Nada
}

// ------------------------------------------------------------------- pruebas
#[cfg(test)]
mod tests {
    use super::*;

    const HOY: (i32, u32, u32) = (2026, 6, 15);

    fn doc(t: &str) -> (String, Option<String>) {
        match interpretar(t, HOY) {
            Idea::Documento { nombre, vence } => (nombre, vence),
            otro => panic!("esperaba documento, salió {otro:?}"),
        }
    }

    #[test]
    fn documentos_con_fecha() {
        assert_eq!(
            doc("el SOAT vence el 15/03/2027"),
            ("SOAT".into(), Some("15/03/2027".into()))
        );
        assert_eq!(
            doc("la tecnomecanica vence el 15 de marzo de 2027"),
            ("Tecnomecánica".into(), Some("15/03/2027".into()))
        );
        assert_eq!(
            doc("el seguro vence en 6 meses"),
            ("Seguro".into(), Some("15/12/2026".into()))
        );
        assert_eq!(
            doc("impuestos vence en marzo"),
            ("Impuestos".into(), Some("01/03/2027".into()))
        );
    }

    #[test]
    fn fechas_varias() {
        assert_eq!(detectar_fecha("vence 15-3-27", HOY), Some("15/03/2027".into()));
        assert_eq!(detectar_fecha("vence el 20/12", HOY), Some("20/12/2026".into()));
        assert_eq!(detectar_fecha("vence el 1/1", HOY), Some("01/01/2027".into()));
        assert_eq!(detectar_fecha("hoy", HOY), Some("15/06/2026".into()));
        assert_eq!(detectar_fecha("manana", HOY), Some("16/06/2026".into()));
        assert_eq!(detectar_fecha("el ano entrante", HOY), Some("15/06/2027".into()));
    }

    #[test]
    fn numeros_colombianos() {
        assert_eq!(a_numero("50.000"), Some(50_000.0));
        assert_eq!(a_numero("13.900"), Some(13_900.0));
        assert_eq!(a_numero("50000"), Some(50_000.0));
        assert_eq!(a_numero("50,5"), Some(50.5));
        assert_eq!(a_numero("2.000.000"), Some(2_000_000.0));
    }

    #[test]
    fn tanqueos() {
        match interpretar("tanquee 50.000 a 13.900 el galon", HOY) {
            Idea::Tanqueo { monto, precio, .. } => {
                assert_eq!(monto, Some(50_000.0));
                assert_eq!(precio, Some(13_900.0));
            }
            otro => panic!("esperaba tanqueo, salió {otro:?}"),
        }
        match interpretar("le puse 50 mil de gasolina", HOY) {
            Idea::Tanqueo { monto, .. } => assert_eq!(monto, Some(50_000.0)),
            otro => panic!("esperaba tanqueo, salió {otro:?}"),
        }
    }

    #[test]
    fn servicios() {
        match interpretar("le hice el cambio de aceite a los 15.000 km", HOY) {
            Idea::Servicio {
                nombre, ultimo_km, ..
            } => {
                assert_eq!(nombre, "Cambio de aceite");
                assert_eq!(ultimo_km, Some(15_000));
            }
            otro => panic!("esperaba servicio, salió {otro:?}"),
        }
        match interpretar("cambie las pastillas de freno, me costo 80.000", HOY) {
            Idea::Servicio { nombre, costo, .. } => {
                assert_eq!(nombre, "Pastillas de freno");
                assert_eq!(costo, Some(80_000.0));
            }
            otro => panic!("esperaba servicio, salió {otro:?}"),
        }
    }

    #[test]
    fn nada() {
        assert_eq!(interpretar("hola como estas", HOY), Idea::Nada);
        assert_eq!(interpretar("", HOY), Idea::Nada);
    }
}
