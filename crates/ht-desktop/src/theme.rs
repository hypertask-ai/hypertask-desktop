use gpui::{Hsla, rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeName {
    Porcelain,
    Graphite,
    Amoled,
    Dia,
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub page: u32,
    pub detail: u32,
    pub container: u32,
    pub card: u32,
    pub comment: u32,
    pub rail: u32,
    pub rail_border: u32,
    pub border: u32,
    pub column_border: u32,
    pub text: u32,
    pub muted: u32,
    pub active: u32,
    pub hover: u32,
    pub footer: u32,
    pub accent: u32,
    pub link: u32,
    pub inbox: u32,
    pub inbox_page: u32,
}

pub const THEMES: [Theme; 4] = [
    Theme {
        page: 0xffffff,
        detail: 0xffffff,
        container: 0xf5f5f6,
        card: 0xffffff,
        comment: 0xf9f9fa,
        rail: 0xf5f5f6,
        rail_border: 0xdcdce1,
        border: 0xe2e2e6,
        column_border: 0xe2e2e6,
        text: 0x18181b,
        muted: 0x727279,
        active: 0xe8e8eb,
        hover: 0xeeeef1,
        footer: 0xececef,
        accent: 0x5c6070,
        link: 0x1365a3,
        inbox: 0xf5f5f6,
        inbox_page: 0xfafafc,
    },
    Theme {
        page: 0x0e0e0e,
        detail: 0x1a1d21,
        container: 0x1a1d21,
        card: 0x262a30,
        comment: 0x262a30,
        rail: 0x1a1d21,
        rail_border: 0x3a3f47,
        border: 0x33373e,
        column_border: 0x33373e,
        text: 0xffffff,
        muted: 0x8e9093,
        active: 0x363b44,
        hover: 0x3a3d45,
        footer: 0x262930,
        accent: 0x707788,
        link: 0x5896f1,
        inbox: 0x1a1d21,
        inbox_page: 0x0e0e0e,
    },
    Theme {
        page: 0x000000,
        detail: 0x000000,
        container: 0x101010,
        card: 0x1d1d1d,
        comment: 0x121212,
        rail: 0x000000,
        rail_border: 0x3a3a3a,
        border: 0x333333,
        column_border: 0x343434,
        text: 0xececec,
        muted: 0x8a8a8a,
        active: 0x282828,
        hover: 0x34363d,
        footer: 0x1d1e23,
        accent: 0x6b7180,
        link: 0x5896f1,
        inbox: 0x000000,
        inbox_page: 0x000000,
    },
    Theme {
        page: 0xf6f4ef,
        detail: 0xf6f4ef,
        container: 0xf6f4ef,
        card: 0xffffff,
        comment: 0xffffff,
        rail: 0xf6f4ef,
        rail_border: 0xe9e4da,
        border: 0xe9e4da,
        column_border: 0xf6f4ef,
        text: 0x1f1d1a,
        muted: 0x67635b,
        active: 0xc9e3d2,
        hover: 0xdeede2,
        footer: 0xece8df,
        accent: 0x3e6b4f,
        link: 0x3e6b4f,
        inbox: 0xffffff,
        inbox_page: 0xf6f4ef,
    },
];

impl ThemeName {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "porcelain" => Ok(Self::Porcelain),
            "graphite" => Ok(Self::Graphite),
            "amoled" => Ok(Self::Amoled),
            "dia" => Ok(Self::Dia),
            _ => anyhow::bail!("unknown theme: {s}"),
        }
    }
    pub fn colors(self) -> Theme {
        THEMES[self as usize]
    }
    pub fn font(self) -> &'static str {
        if self == Self::Dia {
            "Inter"
        } else {
            "IBM Plex Sans"
        }
    }
}

pub fn color(value: u32) -> Hsla {
    rgb(value).into()
}
