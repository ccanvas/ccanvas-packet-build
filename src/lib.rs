use std::{fmt::Write, fs, path::Path};

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn group_id(attr: TokenStream, item: TokenStream) -> TokenStream {
    item.into_iter()
        .chain(
            format!("\n\nimpl Group {{ pub fn id() -> &'static [u8] {{ &[{attr}] }} }}",)
                .parse::<TokenStream>()
                .unwrap(),
        )
        .collect()
}

#[proc_macro]
pub fn declare_packet(_: TokenStream) -> TokenStream {
    let content = fs::read_to_string(Path::new("/tmp").join(format!(
        "ccanvas-packets-index-{}.txt",
        env!("CARGO_PKG_VERSION").replace(".", "-")
    )))
    .unwrap();
    let index = content
        .lines()
        .map(|item| item.split_once('=').unwrap())
        .collect::<Vec<_>>();

    let load_mod = index.iter().fold(String::new(), |mut acc, (item, _)| {
        write!(acc, "#[cfg(feature = \"{item}\")]\npub mod {item};\n").unwrap();
        acc
    });

    let packet_enum = format!(
        "#[cfg_attr(feature = \"debug\", derive(Debug))]\npub enum Packet {{\n{}}}",
        index.iter().fold(String::new(), |mut acc, (item, _)| {
            write!(
                acc,
                "    #[cfg(feature = \"{item}\")]\n    {}{}({item}::Group),\n",
                item[0..1].to_uppercase(),
                &item[1..]
            )
            .unwrap();
            acc
        })
    );

    let parse = format!(r#"impl Packet {{
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {{
        let ident_len = *bytes.first()? as usize;
        
        if bytes.len() <= ident_len {{
            return None;
        }}

        match bytes[1..1+ident_len] {{{}
            _ => None
        }}
    }}
}}"#, index.iter().fold(String::new(), |mut acc, (name, ident)| {
    write!(acc, "\n            #[cfg(feature=\"{name}\")]\n            [{ident}] => Some(Self::{}{}({name}::Group::from_bytes(&bytes[1+ident_len..])?)),", name[0..1].to_uppercase(), &name[1..]).unwrap();
    acc
}));

    let ser = format!(r#"impl Packet {{
    pub fn to_bytes(self) -> Vec<u8> {{
        match self {{{}
        }}
    }}
}}"#, index.iter().fold(String::new(), |mut acc, (name, _)| {
    write!(acc, "\n            #[cfg(feature=\"{name}\")]\n            Self::{}{}(group) => group.to_bytes(),", name[0..1].to_uppercase(), &name[1..]).unwrap();
    acc
}));

    format!("{load_mod}\n\n{packet_enum}\n\n{parse}\n\n{ser}")
        .parse()
        .unwrap()
}
