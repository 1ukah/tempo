pub enum Kv {
    Section(Vec<(String, Kv)>),
    Str(String),
    Int(i64),
    Float(f32),
}

impl Kv {
    pub fn get(&self, key: &str) -> Option<&Kv> {
        self.children()
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    pub fn children(&self) -> &[(String, Kv)] {
        match self {
            Kv::Section(children) => children,
            _ => &[],
        }
    }

    pub fn first_section(&self) -> Option<&Kv> {
        self.children()
            .iter()
            .map(|(_, v)| v)
            .find(|v| matches!(v, Kv::Section(_)))
    }

    pub fn as_text(&self) -> String {
        match self {
            Kv::Str(s) => s.clone(),
            Kv::Int(i) => i.to_string(),
            Kv::Float(f) => f.to_string(),
            Kv::Section(_) => String::new(),
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Kv::Int(i) => u32::try_from(*i).ok(),
            Kv::Str(s) => s.trim().parse().ok(),
            _ => None,
        }
    }
}

pub fn parse(data: &[u8]) -> Option<Kv> {
    let mut pos = 0;
    let mut root = Vec::new();
    section(data, &mut pos, &mut root, 0)?;
    Some(Kv::Section(root))
}

fn section(data: &[u8], pos: &mut usize, out: &mut Vec<(String, Kv)>, depth: u8) -> Option<()> {
    if depth > 32 {
        return None;
    }

    while *pos < data.len() {
        let kind = data[*pos];
        *pos += 1;

        if matches!(kind, 8 | 11) {
            return Some(());
        }

        let key = cstr(data, pos)?;

        let value = match kind {
            0 => {
                let mut children = Vec::new();
                section(data, pos, &mut children, depth + 1)?;
                Kv::Section(children)
            }
            1 => Kv::Str(cstr(data, pos)?),
            2 | 4 | 6 => Kv::Int(i64::from(i32::from_le_bytes(take(data, pos)?))),
            3 => Kv::Float(f32::from_le_bytes(take(data, pos)?)),
            5 => Kv::Str(wide_str(data, pos)?),
            7 => Kv::Int(u64::from_le_bytes(take(data, pos)?) as i64),
            10 => Kv::Int(i64::from_le_bytes(take(data, pos)?)),
            _ => return None,
        };

        out.push((key, value));
    }

    Some(())
}

fn take<const N: usize>(data: &[u8], pos: &mut usize) -> Option<[u8; N]> {
    let bytes = data.get(*pos..*pos + N)?.try_into().ok()?;
    *pos += N;
    Some(bytes)
}

fn cstr(data: &[u8], pos: &mut usize) -> Option<String> {
    let end = *pos + data.get(*pos..)?.iter().position(|&b| b == 0)?;
    let text = String::from_utf8_lossy(&data[*pos..end]).into_owned();
    *pos = end + 1;
    Some(text)
}

fn wide_str(data: &[u8], pos: &mut usize) -> Option<String> {
    let mut units = Vec::new();

    loop {
        let unit = u16::from_le_bytes(take(data, pos)?);

        if unit == 0 {
            return Some(String::from_utf16_lossy(&units));
        }

        units.push(unit);
    }
}
