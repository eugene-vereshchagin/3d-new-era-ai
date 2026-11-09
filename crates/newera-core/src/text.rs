//! A finding's sentence as a template and its data, so it can be said in
//! another language.
//!
//! Findings used to be Portuguese strings built inside the rules and printed
//! raw: there was nothing to translate but the finished sentence. A [`Text`]
//! keeps what the sentence was made of — a template with `{}` where the data
//! goes, and the data — beside the Portuguese it renders to.
//!
//! The Portuguese is the canonical form and does not change. It is rendered
//! by `format!` from the same template and arguments as before, so it is the
//! same string, byte for byte; it is what a finding is compared, merged and
//! accepted by, and what the pre-`Rule` acceptance keys were built from — see
//! `legacy_key_of` in the review. Translating a finding never touches it.
//!
//! Numbers and names travel as they were written in Portuguese (`1,60 m`,
//! `Quarto r5`): a translation says the sentence in another language, it does
//! not reformat the data inside it. A word the sentence is built from — "à
//! frente", "dormitório", "cuba" — is not data: it is a [`Text`] of its own,
//! passed as an argument, and translated with the sentence.

use std::fmt;
use std::ops::Deref;

use crate::vocabulary::Language;

/// One piece of a sentence's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// A number, a name, an id: shown as it is in every language.
    Plain(String),
    /// Words of their own — a side, a room's use, another finding's sentence
    /// — translated with the sentence they sit in. `quoted` drops the closing
    /// period, for a sentence cited inside another.
    Text { text: Box<Text>, quoted: bool },
}

impl Arg {
    /// The piece as it reads in `language`.
    #[must_use]
    pub fn render(&self, language: Language) -> String {
        match self {
            Self::Plain(text) => text.clone(),
            Self::Text { text, quoted } => {
                let said = text.in_language(language);
                if *quoted {
                    said.trim_end_matches('.').to_owned()
                } else {
                    said
                }
            }
        }
    }
}

/// What can fill a hole in a template.
///
/// Data — numbers, names, ids — is shown as written; a [`Text`] is words, and
/// is translated. There is no blanket impl over `Display` on purpose: a
/// Portuguese `&str` passed where a word belongs would compile and never be
/// translated, so every kind of data is named here.
pub trait ToArg {
    fn to_arg(&self) -> Arg;
}

impl ToArg for Text {
    fn to_arg(&self) -> Arg {
        Arg::Text {
            text: Box::new(self.clone()),
            quoted: false,
        }
    }
}

impl<T: ToArg + ?Sized> ToArg for &T {
    fn to_arg(&self) -> Arg {
        (**self).to_arg()
    }
}

macro_rules! plain {
    ($($t:ty),+) => {$(
        impl ToArg for $t {
            fn to_arg(&self) -> Arg {
                Arg::Plain(self.to_string())
            }
        }
    )+};
}

plain!(
    str,
    String,
    f64,
    f32,
    u8,
    u16,
    u32,
    u64,
    usize,
    i32,
    i64,
    char,
    bool,
    crate::ids::FurnitureId,
    crate::ids::RoomId,
    crate::ids::WallId,
    crate::ids::ElementId
);

/// A sentence from a template, with only `{}` for its data.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Part {
    template: &'static str,
    args: Vec<Arg>,
}

/// A sentence: one or more templates with their data, and the Portuguese
/// they render to. Joined parts are separated by a space.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Text {
    parts: Vec<Part>,
    portuguese: String,
}

impl Text {
    /// A template and its data, already rendered in Portuguese by whoever
    /// had the arguments — [`say!`](crate::say) does both at once.
    #[doc(hidden)]
    #[must_use]
    pub fn template(template: &'static str, args: Vec<Arg>, portuguese: String) -> Self {
        Self {
            parts: vec![Part { template, args }],
            portuguese,
        }
    }

    /// The same sentence followed by `more`, after a space.
    #[must_use]
    pub fn then(mut self, more: Self) -> Self {
        if !self.portuguese.is_empty() && !more.portuguese.is_empty() {
            self.portuguese.push(' ');
        }
        self.portuguese.push_str(&more.portuguese);
        self.parts.extend(more.parts);
        self
    }

    /// The canonical sentence: Portuguese, as it has always been written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.portuguese
    }

    /// Words run together with `separator` — a template with two holes,
    /// `"{}, {}"` or `"{} e {}"` — so the joining word is translated too.
    #[must_use]
    pub fn joined(items: Vec<Self>, separator: &'static str) -> Self {
        let mut items = items.into_iter();
        let Some(first) = items.next() else {
            return Self::default();
        };
        items.fold(first, |so_far, next| {
            let args = vec![so_far.to_arg(), next.to_arg()];
            let shown: Vec<String> = args
                .iter()
                .map(|a| a.render(Language::Portuguese))
                .collect();
            Self::template(separator, args, fill(separator, &shown))
        })
    }

    /// This sentence as an argument of another, without its closing period:
    /// what `«{}»` quotes.
    #[must_use]
    pub fn quoted(&self) -> Quoted {
        Quoted(self.clone())
    }

    /// Whether every part of it can be said in English.
    #[must_use]
    pub fn translatable(&self) -> bool {
        self.parts.iter().all(|part| {
            english(part.template).is_some()
                && part.args.iter().all(|a| match a {
                    Arg::Plain(_) => true,
                    Arg::Text { text, .. } => text.translatable(),
                })
        })
    }

    /// The templates it is made of, quoted ones included, for the tests that
    /// hold the tables whole.
    #[must_use]
    pub fn templates(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for part in &self.parts {
            out.push(part.template);
            for arg in &part.args {
                if let Arg::Text { text, .. } = arg {
                    out.extend(text.templates());
                }
            }
        }
        out
    }

    /// The sentence in `language`, or in Portuguese where there is no
    /// translation.
    #[must_use]
    pub fn in_language(&self, language: Language) -> String {
        if language == Language::Portuguese {
            return self.portuguese.clone();
        }
        let mut out = String::new();
        for part in &self.parts {
            if !out.is_empty() {
                out.push(' ');
            }
            let args: Vec<String> = part.args.iter().map(|a| a.render(language)).collect();
            let template = english(part.template).unwrap_or(part.template);
            out.push_str(&fill(template, &args));
        }
        out
    }
}

/// A sentence cited inside another — see [`Text::quoted`].
#[derive(Debug, Clone)]
pub struct Quoted(Text);

impl ToArg for Quoted {
    fn to_arg(&self) -> Arg {
        Arg::Text {
            text: Box::new(self.0.clone()),
            quoted: true,
        }
    }
}

/// `template` with each `{}` replaced by the next of `args`.
#[doc(hidden)]
#[must_use]
pub fn fill(template: &str, args: &[String]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    let mut args = args.iter();
    while let Some(at) = rest.find("{}") {
        out.push_str(&rest[..at]);
        out.push_str(args.next().map_or("", String::as_str));
        rest = &rest[at + 2..];
    }
    out.push_str(rest);
    out
}

/// How many `{}` a template has — and `None` when it has any other kind of
/// placeholder, which a translation could not follow.
#[must_use]
pub fn holes(template: &str) -> Option<usize> {
    let holes = template.matches("{}").count();
    let whole = template.matches('{').count() == holes && template.matches('}').count() == holes;
    whole.then_some(holes)
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        &self.portuguese
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.portuguese)
    }
}

impl serde::Serialize for Text {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.portuguese)
    }
}

impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        self.portuguese == other
    }
}

impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        self.portuguese == *other
    }
}

/// A fixed sentence, with no data: a template of its own.
impl From<&'static str> for Text {
    fn from(template: &'static str) -> Self {
        Self::template(template, Vec::new(), template.to_owned())
    }
}

/// A sentence from a template: `say!("{} cm livres", cm(free))`.
///
/// The Portuguese is rendered by `format!` from the very same arguments, so it
/// is exactly what `format!` would have made. Only `{}` may appear in the
/// template — a translation follows the holes in order — and every argument is
/// shown with `Display`.
#[macro_export]
macro_rules! say {
    ($template:literal $(,)?) => {
        $crate::text::Text::from($template)
    };
    ($template:literal, $($arg:expr),+ $(,)?) => {{
        // Never run: it makes the compiler count the arguments against the
        // template's holes, as `format!` does.
        if false {
            let _ = ::std::format!($template, $($crate::text::ToArg::to_arg(&$arg).render($crate::vocabulary::Language::Portuguese)),+);
        }
        let args: ::std::vec::Vec<$crate::text::Arg> = ::std::vec![
            $($crate::text::ToArg::to_arg(&$arg)),+
        ];
        let shown: ::std::vec::Vec<::std::string::String> = args
            .iter()
            .map(|a| a.render($crate::vocabulary::Language::Portuguese))
            .collect();
        let portuguese = $crate::text::fill($template, &shown);
        $crate::text::Text::template($template, args, portuguese)
    }};
}

/// The English of a template, where there is one.
#[must_use]
pub fn english(template: &str) -> Option<&'static str> {
    ENGLISH
        .iter()
        .find(|(pt, _)| *pt == template)
        .map(|(_, en)| *en)
}

/// Every finding's sentence, Portuguese to English.
///
/// Kept whole by tests: every template a rule can say has a row here, and
/// each row has as many holes on both sides.
pub static ENGLISH: &[(&str, &str)] = &[
    (
        "Uso do ambiente como {} conflita com equipamentos sanitários ({}). Confirme o programa e declare room_use; o nome e as peças foram preservados.",
        "Using the room as {} conflicts with sanitary fixtures ({}). Confirm the programme and declare room_use; the name and the pieces were kept.",
    ),
    (
        "Pia de cozinha {} usada como lavatório: renomear não altera o equipamento. Confirme se este catálogo corresponde à peça pretendida para o banheiro.",
        "Kitchen sink {} used as a basin: renaming it does not change the fixture. Confirm this catalog item is the piece meant for the bathroom.",
    ),
    (
        "{} moradores e {} lugares para dormir: {} (camas de casal contam 2).",
        "{} residents and {} places to sleep: {} (double beds count as 2).",
    ),
    ("falta 1", "1 missing"),
    ("faltam {}", "{} missing"),
    (
        "{} moradores por dormitório: acima de {} é adensamento excessivo; são necessários {} dormitórios.",
        "{} residents per bedroom: above {} is overcrowding; {} bedrooms are needed.",
    ),
    (
        "{} moradores por dormitório; com {} dormitórios ninguém divide com mais de uma pessoa.",
        "{} residents per bedroom; with {} bedrooms nobody shares with more than one person.",
    ),
    (
        "{} moradores para {} banheiro(s): acima de {} por banheiro as filas de manhã são certas; considere um lavabo ou mais um banheiro.",
        "{} residents for {} bathroom(s): above {} per bathroom there will be queues in the morning; consider a powder room or another bathroom.",
    ),
    (
        "{} lugares à mesa para {} moradores: use uma mesa de {} lugares.",
        "{} seats at the table for {} residents: use a table for {}.",
    ),
    (
        "{} lugares na sala para {} moradores: todos sentam juntos com mais {} lugar(es) (poltrona ou sofá maior).",
        "{} seats in the living room for {} residents: everyone sits together with {} more seat(s) (an armchair or a larger sofa).",
    ),
    (
        "{} cm de guarda-roupa nos dormitórios e closets; a norma de desempenho (anexo F) prevê 1,60 m no dormitório de casal e 1,20 m no de solteiro, uns {} cm por adulto e metade por criança: {} cm no total.",
        "{} cm of wardrobe in bedrooms and closets; the performance standard (annex F) allows 1.60 m in a couple's bedroom and 1.20 m in a single one, some {} cm per adult and half per child: {} cm in all.",
    ),
    ("à frente", "in front"),
    ("à esquerda", "on the left"),
    ("à direita", "on the right"),
    ("afaste {} {} cm", "move {} {} cm away"),
    ("afaste {} cm", "move it {} cm away"),
    (
        "não há espaço do outro lado: use peça menor ou reorganize",
        "there is no room on the other side: use a smaller piece or rearrange",
    ),
    (" em {} dos {} cm", " along {} of its {} cm"),
    (
        "{} cm livres {}{} ({}: mínimo {} cm); {}.",
        "{} cm free {}{} ({}: at least {} cm); {}.",
    ),
    (
        "Vão livre de cerca de {} cm: abaixo de {} cm não passa um móvel nem uma pessoa com volumes; use porta de 70 cm ou mais.",
        "Clear opening of about {} cm: below {} cm neither furniture nor a person carrying things gets through; use a door of 70 cm or more.",
    ),
    (
        "Vão livre de cerca de {} cm: cadeira de rodas e andador pedem {} cm livres; use porta de 90 cm.",
        "Clear opening of about {} cm: a wheelchair or a walker needs {} cm clear; use a 90 cm door.",
    ),
    ("banheiro", "bathroom"),
    ("dormitório", "bedroom"),
    (
        "Sem acesso: nenhuma porta nem vão chega ao {}; sem uma porta, ele não pode ser usado.",
        "No access: no door or opening reaches the {}; without a door it cannot be used.",
    ),
    (
        "Sem porta: o único acesso ao {} é um vão livre ({}), que não fecha nem dá privacidade; troque por uma porta.",
        "No door: the only way into the {} is an open passage ({}), which neither closes nor gives privacy; replace it with a door.",
    ),
    (
        "{} {} parece a folha desenhada ao lado, sem ser uma abertura.",
        "{} {} looks like the leaf drawn beside it, without being an opening.",
    ),
    (
        "A folha da porta bate em {}: com a dobradiça do outro lado ela abre livre (hinge_right hoje {}, use {}).",
        "The door leaf hits {}: hinged on the other side it opens clear (hinge_right is {} now, use {}).",
    ),
    (
        "A folha da porta bate em {}: movendo a peça {} cm a porta abre livre.",
        "The door leaf hits {}: moving the piece {} cm lets the door open clear.",
    ),
    (
        "A folha da porta bate em {}: mova a peça ou use porta de correr.",
        "The door leaf hits {}: move the piece or use a sliding door.",
    ),
    (
        "Ocupa o mesmo lugar que {}: movendo {} {} cm fica livre.",
        "Takes the same place as {}: moving {} {} cm frees it.",
    ),
    (
        "Ocupa o mesmo lugar que {}: não há lugar livre por perto, reorganize.",
        "Takes the same place as {}: there is no free spot nearby, rearrange.",
    ),
    (
        "{} m² para {}: códigos de obras costumam pedir ao menos {} m² (confira o do seu município).",
        "{} m² for a {}: building codes usually ask for at least {} m² (check your municipality's).",
    ),
    (
        "Menor lado de {} cm; para {} a referência é pelo menos {} cm.",
        "Shortest side of {} cm; for a {} the reference is at least {} cm.",
    ),
    (
        "Pé-direito de {} cm; o mínimo é {} cm em {}.",
        "Ceiling height of {} cm; the minimum is {} cm in a {}.",
    ),
    (
        "use janela basculante ou ventilação mecânica",
        "use a hopper window or mechanical ventilation",
    ),
    (
        "precisa de janela para luz e ventilação",
        "it needs a window for light and air",
    ),
    ("Sem janela: {}.", "No window: {}."),
    (
        "Janelas somam {} m² para {} m² de piso; {} pede 1/{} do piso: {} m², com metade abrindo para ventilar.",
        "Windows add up to {} m² for {} m² of floor; {} asks for 1/{} of the floor: {} m², half of it opening for air.",
    ),
    (
        "Janelas somam {} m² para {} m² de piso; o Código Sanitário de SP pede 1/{} do piso: {} m², com metade abrindo para ventilar.",
        "Windows add up to {} m² for {} m² of floor; São Paulo's Sanitary Code asks for 1/{} of the floor: {} m², half of it opening for air.",
    ),
    (
        "Falta {} para testar o uso de {}.",
        "Missing {} to try out the use of a {}.",
    ),
    (
        "Cabe um giro de {} cm; são necessários {} cm livres para girar a cadeira de rodas.",
        "A {} cm turn fits; {} cm clear are needed to turn a wheelchair.",
    ),
    (
        "Triângulo geladeira–pia–fogão de {} cm, lado maior {} cm: a referência é até {} cm no total e {} cm por lado; aproxime os três.",
        "Fridge–sink–stove triangle of {} cm, longest leg {} cm: the reference is up to {} cm in all and {} cm per leg; bring the three closer.",
    ),
    (
        "Triângulo geladeira–pia–fogão com um lado de só {} cm: a referência é ao menos {} cm, para haver bancada de apoio entre eles.",
        "Fridge–sink–stove triangle with a leg of only {} cm: the reference is at least {} cm, so there is counter to set things on between them.",
    ),
    (
        "Bancada a {} cm; para quem tem {} cm de altura o conforto fica perto de {} cm.",
        "Counter at {} cm; for someone {} cm tall the comfortable height is about {} cm.",
    ),
    (
        "Aéreo a {} cm do chão: abaixo de ~{} cm (45 cm sobre a bancada, prática de marcenaria) a cabeça bate ao trabalhar.",
        "Wall cabinet {} cm off the floor: below ~{} cm (45 cm above the counter, joinery practice) you hit your head while working.",
    ),
    (
        "Topo do aéreo a {} cm: a prateleira de cima fica fora do alcance (~{} cm); guarde ali o que se usa pouco.",
        "Top of the wall cabinet at {} cm: the upper shelf is out of reach (~{} cm); keep what is seldom used there.",
    ),
    (
        "{} cm de bancada livre fora de pia, fogão e geladeira; abaixo de {} cm falta onde pousar as coisas do preparo (mesa solta também conta).",
        "{} cm of free counter besides the sink, stove and fridge; below {} cm there is nowhere to set down what you prepare (a free-standing table counts too).",
    ),
    (
        "Um trecho de bancada de só {} cm: abaixo de {} cm o pedaço não serve para preparar nada; junte-o a outro trecho.",
        "A stretch of counter of only {} cm: below {} cm it is too short to prepare anything on; join it to another stretch.",
    ),
    (
        "{} cm entre {} e {}: acima de {} cm cada par vira travessia, e o preparo se desfaz em idas e vindas.",
        "{} cm between the {} and the {}: above {} cm each pair becomes a crossing, and cooking falls apart in comings and goings.",
    ),
    (
        "Das cinco zonas de trabalho falta {}: sem ela o fluxo mantimentos → armazenagem → lavagem → preparo → cocção se quebra.",
        "Of the five work zones, {} is missing: without it the flow provisions → storage → washing → preparation → cooking breaks.",
    ),
    (
        "{} ponto(s) de tomada para {} m de perímetro: são necessários {} (um a cada {} m ou fração).",
        "{} outlet point(s) for {} m of perimeter: {} are required (one every {} m or part of it).",
    ),
    (
        "{} tomada(s) acima da bancada: são exigidas pelo menos {}, no mesmo ponto (tomada dupla: elec:sockets = 2) ou em pontos distintos; forno, cooktop elétrico e lava-louças acima de 10 A pedem circuito próprio.",
        "{} outlet(s) above the counter: at least {} are required, at one point (a double outlet: elec:sockets = 2) or at separate points; an oven, electric cooktop or dishwasher above 10 A needs a circuit of its own.",
    ),
    (
        "Coifa de {} cm sobre cocção de {} cm: a captura já cai à metade nas bocas da frente, e vazão alta não compensa coifa estreita.",
        "A {} cm hood over a {} cm cooktop: capture already halves at the front burners, and a high flow rate does not make up for a narrow hood.",
    ),
    (
        "Cozinha de {} cm de largura; para comparar, a unidade do Minha Casa Minha Vida (regra só para ela) entrega {} cm, com previsão de pia 120×50, fogão 55×60 e geladeira 70×70 cm.",
        "Kitchen {} cm wide; for comparison, a Minha Casa Minha Vida unit (a rule for it alone) is delivered at {} cm, with room for a 120×50 sink, a 55×60 stove and a 70×70 cm fridge.",
    ),
    (
        "Cabe um círculo de {} cm no piso; {} pede {} cm ({}). Entre norma e lei local prevalece o mais restritivo.",
        "A {} cm circle fits on the floor; {} asks for {} cm ({}). Between a standard and local law, the stricter one prevails.",
    ),
    (
        "{} a {} cm da TV de {}\"; para essa tela o conforto fica entre {} e {} cm.",
        "{} at {} cm from the {}\" TV; for that screen the comfortable range is between {} and {} cm.",
    ),
    (
        "{} cm entre {} e {}: entre duas camas de solteiro a referência é {} cm.",
        "{} cm between {} and {}: between two single beds the reference is {} cm.",
    ),
    (
        "Tampo a {} cm: para cadeira de rodas, até {} cm, com vão livre embaixo.",
        "Top at {} cm: for a wheelchair, up to {} cm, with clear space underneath.",
    ),
    (
        "Cama a {} cm do chão: a transferência da cadeira pede uns {} cm.",
        "Bed {} cm off the floor: a transfer from the wheelchair needs about {} cm.",
    ),
    (
        "{} interruptor(es)/tomada(s) fora do alcance de quem usa cadeira de rodas: interruptores entre {} e {} cm, tomadas entre {} e {} cm.",
        "{} switch(es)/outlet(s) out of reach from a wheelchair: switches between {} and {} cm, outlets between {} and {} cm.",
    ),
    (
        "Não cabem os dois: afastar isso cria «{}» em {}; a posição atual é a melhor das duas — aceite o que ficar com o motivo.",
        "The two do not fit: moving this creates «{}» at {}; the current position is the better of the two — accept whichever stays, with the reason.",
    ),
    ("Isso deixa «{}» em {}{}.", "That leaves «{}» at {}{}."),
    (", já aceito", ", already accepted"),
    (", mais leve", ", which is lighter"),
    (
        "Possível canto ocioso junto a ({}, {}) cm: {} × {} cm ({} m²), entre {} e {}. Há uma sobra delimitada pelos móveis e pelas paredes, com acesso estreito. Feedback de aproveitamento, não defeito confirmado: confira se é folga de ventilação, manutenção ou abertura antes de decidir usá-la. Nenhuma alteração automática.",
        "Possible idle corner near ({}, {}) cm: {} × {} cm ({} m²), between {} and {}. There is leftover space enclosed by furniture and walls, with narrow access. Feedback on use of space, not a confirmed defect: check whether it is clearance for ventilation, maintenance or an opening before deciding to use it. Nothing is changed automatically.",
    ),
    (
        "Sem espaço embaixo: o corpo da torre desce uns {} cm sob o tampo e cai sobre {} ({}); mova-a para cima de um vão livre do gabinete.",
        "No room underneath: the tower's body goes some {} cm below the top and lands on {} ({}); move it over a free space in the cabinet.",
    ),
    (
        "A {} cm da borda do tampo: os fabricantes pedem ao menos 2,5 cm entre o furo e a borda.",
        "{} cm from the edge of the top: manufacturers ask for at least 2.5 cm between the hole and the edge.",
    ),
    (
        "A {} cm da {}: mantenha a torre a 30 cm da cuba e do cooktop, fora da área de respingo e de calor.",
        "{} cm from the {}: keep the tower 30 cm from the sink bowl and the cooktop, out of the splash and heat zone.",
    ),
    ("cuba", "sink bowl"),
    ("cocção", "cooking"),
    (
        "{} W de iluminação no cômodo e o dimmer aguenta {} W: divida as luminárias em dois dimmers ou use um de maior capacidade.",
        "{} W of lighting in the room and the dimmer takes {} W: split the fixtures over two dimmers or use a larger one.",
    ),
    (
        "{} W de iluminação para dimerizar: abaixo da carga mínima (em geral 10 W) o LED pisca ou não apaga de todo; confira se as lâmpadas são dimerizáveis.",
        "{} W of lighting to dim: below the minimum load (usually 10 W) LEDs flicker or never quite switch off; check that the lamps are dimmable.",
    ),
    (
        "Sensor de teto a {} cm: os fabricantes o instalam por volta de 2,4 m (até 2,9 m); fora disso o alcance muda; na parede, use o sensor de parede.",
        "Ceiling sensor at {} cm: manufacturers install it at about 2.4 m (up to 2.9 m); outside that its range changes; on a wall, use the wall sensor.",
    ),
    (
        "Não vê o cômodo inteiro: {} m até o canto mais longe e alcance de uns {} m; centralize-o ou ponha um segundo sensor.",
        "It does not see the whole room: {} m to the farthest corner and a range of about {} m; center it or add a second sensor.",
    ),
    (
        "um a cada 3,5 m de perímetro ({} m)",
        "one every 3.5 m of perimeter ({} m)",
    ),
    ("um junto ao lavatório", "one next to the basin"),
    ("ao menos um", "at least one"),
    (
        "um a cada 5 m de perímetro ({} m)",
        "one every 5 m of perimeter ({} m)",
    ),
    (
        "{} de {} tomadas de uso geral: a norma pede {}.",
        "{} of {} general-purpose outlets: the standard asks for {}.",
    ),
    (
        "{} de {} pontos de rede RJ45: a norma de cabeamento residencial recomenda {} neste cômodo, cada um com cabo de 4 pares até o distribuidor e uma tomada de energia ao lado.",
        "{} of {} RJ45 network points: the residential cabling standard recommends {} in this room, each with a 4-pair cable to the distributor and a power outlet beside it.",
    ),
    (
        "{} de {} pontos de TV: a norma de cabeamento residencial recomenda {} neste cômodo (coaxial até 100 m do distribuidor).",
        "{} of {} TV points: the residential cabling standard recommends {} in this room (coaxial, up to 100 m from the distributor).",
    ),
    (
        "Pontos sem cabo chegando: {}.",
        "Points with no cable reaching them: {}.",
    ),
    (
        "{} sai com uplink de {}: o cabo que chega é {} e o que sustenta essa velocidade é {}.",
        "{} comes with an uplink of {}: the cable that reaches it is {} and what sustains that speed is {}.",
    ),
    (
        "Não cabe: {} módulos DIN (com {} de reserva) num quadro de {} módulos{}. Use um quadro maior ou divida em dois, antes de a parede ser fechada.",
        "It does not fit: {} DIN modules (with {} spare) in a panel of {} modules{}. Use a larger panel or split it in two, before the wall is closed.",
    ),
    (
        " (estimado pelo tamanho; informe elec:modules)",
        " (estimated from its size; set elec:modules)",
    ),
    (
        "Geral de {} A e parcial de {} A: com menos de o dobro, uma falta no circuito maior pode desarmar o geral junto (regra prática, não da norma). Confira a seletividade nas tabelas do fabricante; geral curva C e parciais curva B ajudam.",
        "Main breaker of {} A and branch of {} A: with less than double, a fault on the larger circuit can trip the main as well (a rule of thumb, not the standard's). Check selectivity in the manufacturer's tables; a C-curve main and B-curve branches help.",
    ),
    (
        "Capacidade de interrupção dos disjuntores assumida em {} kA, o que a Enel SP pede até 63 A: confirme com a concessionária a corrente de curto presumida no ponto de entrega e informe elec:short_ka.",
        "Breaker breaking capacity assumed at {} kA, what Enel SP asks for up to 63 A: confirm the prospective short-circuit current at the service point with the utility and set elec:short_ka.",
    ),
    (
        "{} pontos sem circuito: {}.",
        "{} points with no circuit: {}.",
    ),
    (
        "carrega {} A, acima dos 16 A",
        "it carries {} A, above 16 A",
    ),
    (
        "toda a iluminação ficou em circuitos mistos",
        "all of the lighting ended up on mixed circuits",
    ),
    (
        "todas as tomadas ficaram em circuitos mistos",
        "all of the outlets ended up on mixed circuits",
    ),
    (
        "Iluminação e tomadas no mesmo circuito: a norma só admite em residência até 16 A e sem que toda a iluminação ou todas as tomadas fiquem em circuitos mistos (9.5.3.3); aqui {}.",
        "Lighting and outlets on the same circuit: in a home the standard only allows it up to 16 A and without all the lighting or all the outlets being on mixed circuits (9.5.3.3); here {}.",
    ),
    (
        "{} passa de 10 A e pede circuito exclusivo (9.5.3.1).",
        "{} exceeds 10 A and needs a dedicated circuit (9.5.3.1).",
    ),
    (
        "Queda de tensão de {} % até o ponto mais longe: o circuito terminal pede no máximo 4 % (6.2.7.2); aumente a seção de {} mm² ou divida o circuito.",
        "Voltage drop of {} % to the farthest point: a final circuit allows at most 4 % (6.2.7.2); increase the {} mm² conductor size or split the circuit.",
    ),
    (
        "Precisa de neutro na caixa: numa reforma confira, porque a instalação antiga costuma levar só fase e retorno ao interruptor; no projeto, leve o neutro até ela. Aceite quando o neutro estiver garantido.",
        "It needs a neutral in the box: in a renovation check it, because old wiring usually brings only the line and the switch leg to the switch; in the design, run the neutral to it. Accept once the neutral is assured.",
    ),
    (
        "Fechadura eletrônica fora de uma porta: ponha-a na folha que ela tranca.",
        "Electronic lock not on a door: put it on the leaf it locks.",
    ),
    (
        "Sem ponto de luz: a norma pede ao menos um ponto de iluminação no teto de cada cômodo, comandado por interruptor.",
        "No light point: the standard asks for at least one ceiling lighting point in every room, controlled by a switch.",
    ),
    (
        "Distribuidor de telecom sem tomada de energia junto: modem, roteador e switch precisam dela.",
        "Telecom distributor with no power outlet beside it: the modem, router and switch need one.",
    ),
    (
        "Há pontos de rede, TV ou Wi-Fi e nenhum quadro de telecom: os cabos precisam de um ponto de distribuição que os reúna.",
        "There are network, TV or Wi-Fi points and no telecom panel: the cables need a distribution point to gather them.",
    ),
    (
        "Há cargas e nenhum quadro de distribuição.",
        "There are loads and no distribution panel.",
    ),
    (
        "Nenhum cabo chega ao quadro de telecom: os pontos precisam ser levados até ele.",
        "No cable reaches the telecom panel: the points have to be run to it.",
    ),
    (
        "Access point sem alimentação: nenhuma tomada a até 1,5 m. Alimente por PoE (switch ou injetor PoE+ 802.3at no rack para Wi-Fi 6, 6E e 7 doméstico; 802.3bt para Wi-Fi 7 corporativo; e marque poe) ou ponha uma tomada no forro junto a ele (distância de referência, não de norma).",
        "Access point with no power: no outlet within 1.5 m. Power it over PoE (a PoE+ 802.3at switch or injector in the rack for home Wi-Fi 6, 6E and 7; 802.3bt for enterprise Wi-Fi 7; and mark poe) or put an outlet in the ceiling void beside it (a reference distance, not a standard's).",
    ),
    (
        "Tomadas de cozinha, copa, lavanderia ou área de serviço dividem o circuito com iluminação ou com pontos de outros cômodos: elas pedem circuitos só delas (9.5.3.2).",
        "Outlets of a kitchen, pantry, laundry or service area share a circuit with lighting or with points of other rooms: they need circuits of their own (9.5.3.2).",
    ),
    (
        "Tomada embutida em móvel no banheiro: as torres são IPX3 no máximo (respingo leve) e nenhuma é para área molhada.",
        "Outlet built into furniture in the bathroom: towers are IPX3 at most (light splashing) and none is made for a wet area.",
    ),
    (
        "Torre de plugue (cabo de 1,5 m): preveja uma tomada dentro do gabinete, num circuito com DR, para ligá-la sem extensão.",
        "Plug-in tower (1.5 m cord): plan an outlet inside the cabinet, on an RCD-protected circuit, to plug it in without an extension.",
    ),
    (
        "Sem ponto de água fria a até {} cm: {} não tem de onde receber água.",
        "No cold water point within {} cm: {} has nowhere to get water from.",
    ),
    (
        "O projeto tem água quente e {} não recebe: falta o ponto a até {} cm.",
        "The design has hot water and {} gets none: the point within {} cm is missing.",
    ),
    (
        "ponto de esgoto ou ralo sifonado",
        "sewer point or trapped floor drain",
    ),
    ("ponto de esgoto próprio", "sewer point of its own"),
    (
        "Sem {} a até {} cm: {} não tem para onde escoar (ramal de {} mm).",
        "No {} within {} cm: {} has nowhere to drain to ({} mm branch).",
    ),
    (
        "Nenhum ralo do cômodo é desconector: ralo seco, linear sem sifão ou sifonado pequeno (fecho de 9 a 20 mm) precisam desaguar numa caixa sifonada com fecho de 50 mm ({}).",
        "None of the room's drains is a trap: a dry drain, an untrapped linear drain or a small trapped one (9 to 20 mm seal) must discharge into a trap box with a 50 mm seal ({}).",
    ),
    (
        "{} UHC vão para o ralo e ele aguenta {}: a saída de 50 mm leva até 6 UHC; use a caixa sifonada 150×185×75 (até 15), divida entre duas caixas ou leve aparelhos a ramais próprios.",
        "{} fixture units go to the drain and it takes {}: a 50 mm outlet carries up to 6; use the 150×185×75 trap box (up to 15), split between two boxes or give fixtures branches of their own.",
    ),
    (
        "A {} cm do tubo ventilador mais próximo: um ramal de {} mm pede ventilação a até {} cm (em linha reta; trace o ramal com route kind=vent e ele passa a contar).",
        "{} cm from the nearest vent pipe: a {} mm branch needs venting within {} cm (in a straight line; draw the branch with route kind=vent and it counts).",
    ),
    (
        "Caixa de inspeção de {} cm: a norma pede ao menos 60 cm de lado ou diâmetro, e até 1 m de profundidade (mais funda é poço de visita).",
        "Inspection box of {} cm: the standard asks for at least 60 cm side or diameter, and up to 1 m deep (deeper is a manhole).",
    ),
    (
        "A {} m da caixa de inspeção: vaso, caixa sifonada e caixa de gordura ficam a até 10 m de um dispositivo de inspeção.",
        "{} m from the inspection box: toilets, trap boxes and grease traps stay within 10 m of an inspection device.",
    ),
    (
        "{} linha(s) desenhadas à mão sem dizer se são água fria, quente ou esgoto ({}): não entram nos metros de tubo; trace com route (que as substitui) ou apague.",
        "{} hand-drawn line(s) that do not say whether they are cold water, hot water or sewer ({}): they are left out of the pipe lengths; draw them with route (which replaces them) or delete them.",
    ),
    (
        "Pontos sem tubulação chegando: {}.",
        "Points with no pipe reaching them: {}.",
    ),
    (
        "Nenhum ralo dentro da área do box: a água do banho escorre para o resto do banheiro; ponha o ralo (ou um linear) dentro do box, com caimento de 1,5 % a 2,5 % para ele.",
        "No drain inside the shower area: the shower water runs into the rest of the bathroom; put the drain (or a linear one) inside the shower, with a 1.5 % to 2.5 % fall towards it.",
    ),
    (
        "Área descoberta com ralo de esgoto: a água de chuva vai para o sistema pluvial, nunca para o esgoto; use o ralo pluvial.",
        "Uncovered area with a sewer drain: rainwater goes to the storm system, never to the sewer; use a storm drain.",
    ),
    (
        "Sem caixa de gordura: numa casa, uma pequena (18 L) ou simples (31 L) entre a pia e a rede; em prédio a pia desce por tubo de queda próprio até a caixa coletiva, e caixa individual no andar é vedada — nesse caso, aceite com esse motivo.",
        "No grease trap: in a house, a small (18 L) or simple (31 L) one between the sink and the sewer; in a building the sink goes down its own stack to the shared trap, and a trap on each floor is not allowed — in that case, accept with that reason.",
    ),
    (
        "Sem ventilação: o esgoto pede ao menos um tubo ventilador prolongado acima da cobertura (em prédio, a coluna de ventilação no shaft); sem ele os fechos hídricos se rompem e o cheiro volta.",
        "No venting: the sewer needs at least one vent pipe carried above the roof (in a building, the vent stack in the shaft); without it the water seals break and the smell comes back.",
    ),
    (
        "De onde vem a água? Coloque o hidrômetro ou o registro geral (em prédio, junto à prumada no shaft): é deles que os ramais saem.",
        "Where does the water come from? Place the water meter or the main valve (in a building, by the riser in the shaft): the branches start from there.",
    ),
    (
        "Para onde vai o esgoto? Coloque a caixa de inspeção (em prédio, o tubo de queda no shaft) — é a ela que os ramais descem com caimento.",
        "Where does the sewage go? Place the inspection box (in a building, the stack in the shaft) — that is where the branches fall to.",
    ),
    (
        "Sem ralo: a área do box pede um ralo sifonado (caixa sifonada), que também recebe o lavatório.",
        "No drain: the shower area needs a trapped drain (a trap box), which also takes the basin.",
    ),
    (
        "Sem ralo no piso: o Código Sanitário de SP obriga captação de água no piso de banheiros, cozinhas, copas e lavanderias (pode ser ralo seco).",
        "No floor drain: São Paulo's Sanitary Code requires a floor drain in bathrooms, kitchens, pantries and laundries (it may be a dry drain).",
    ),
    (
        "{} com {} cm: o guarda-corpo tem ao menos 1,10 m do piso ao topo do corrimão; sobre mureta, também 0,90 m acima dela.",
        "{} at {} cm: a guard is at least 1.10 m from the floor to the top of the rail; on a low wall, also 0.90 m above it.",
    ),
    (
        "Vão de {} cm entre as barras: o máximo é 11 cm, e até 45 cm do piso nada em que se possa apoiar o pé para escalar.",
        "A {} cm gap between the bars: the maximum is 11 cm, and up to 45 cm from the floor nothing a foot could climb on.",
    ),
    (
        "Guarda-corpo em vidro {}: a norma pede vidro laminado de segurança (classe 1), aramado ou insulado feito deles; temperado sozinho, ao quebrar, deixa o vão aberto.",
        "Glass guard in {} glass: the standard asks for laminated safety glass (class 1), wired glass or insulating glass made of them; tempered glass alone leaves the opening open when it breaks.",
    ),
    (
        "Fechamento de sacada em vidro {}: pede vidro de segurança temperado ou laminado, com espessura calculada pelo vento do local.",
        "Balcony glazing in {} glass: it needs tempered or laminated safety glass, with a thickness worked out for the local wind.",
    ),
    (
        "O fechamento de vidro está sobre uma mureta de {} cm: sem gradil, a mureta é o guarda-corpo e pede 1,10 m; o vidro não conta.",
        "The glazing stands on a {} cm low wall: with no railing, the low wall is the guard and needs 1.10 m; the glass does not count.",
    ),
    (
        "Diga o vidro do guarda-corpo (glass:type): ele precisa ser laminado de segurança, em geral 4+4 ou 5+5 mm calculado pela norma.",
        "Say what the guard's glass is (glass:type): it must be laminated safety glass, usually 4+4 or 5+5 mm worked out by the standard.",
    ),
    (
        "Fechamento de vidro sem guarda-corpo junto: o envidraçamento não faz a função de guarda-corpo; a sacada precisa do gradil ou do guarda-corpo de vidro, com 1,10 m, atrás ou à frente dele.",
        "Glazing with no guard beside it: glazing does not act as a guard; the balcony needs the railing or the glass guard, 1.10 m high, behind or in front of it.",
    ),
    (
        "{} {} está solta: vai embutida no tampo de uma bancada, ilha ou móvel (ou numa mesa, a caixa de mesa).",
        "{} {} is loose: it is built into the top of a counter, island or cabinet (or, on a table, the table box).",
    ),
    (
        "{} {} vai no teto e está fora de qualquer cômodo.",
        "{} {} goes on the ceiling and is outside every room.",
    ),
    (
        "{} {} está a {} cm do chão, solto no ar: vai fixado no teto, a {} cm.",
        "{} {} is {} cm off the floor, floating in the air: it is fixed to the ceiling, at {} cm.",
    ),
    (
        "{} {} está no {} ({} {}): guarda-corpo e fechamento de sacada não recebem caixa nem ponto; leve-o a uma parede de alvenaria da sacada.",
        "{} {} is on the {} ({} {}): guards and balcony glazing take no box or point; move it to a masonry wall of the balcony.",
    ),
    (
        "{} {} está sobre o vidro de {} ({}): não há onde embutir a caixa; leve-o a uma parede.",
        "{} {} is on the glass of {} ({}): there is nowhere to set the box; move it to a wall.",
    ),
    (
        "{} {} está numa parede de vidro ({}): não há onde embutir a caixa; leve-o a uma parede de alvenaria ou drywall.",
        "{} {} is on a glass wall ({}): there is nowhere to set the box; move it to a masonry or drywall wall.",
    ),
    ("no vão da porta", "in the door opening"),
    ("sobre o vidro da janela", "on the window glass"),
    ("no vão aberto", "in the open passage"),
    ("colado ao batente da porta", "against the door frame"),
    (
        "colado ao batente ou ao peitoril da janela",
        "against the window frame or sill",
    ),
    (
        "{} {} está {} {} ({}): a caixa fica a pelo menos {} cm do vão, fora do batente e do alizar; mova-o para o lado ou abaixo do peitoril.",
        "{} {} is {} of {} ({}): the box stays at least {} cm from the opening, clear of the frame and the casing; move it aside or below the sill.",
    ),
    (
        "{} {} está dentro de {} ({}): tomada de eletrodoméstico vai ao lado dele ou acima do seu topo ({} cm), onde a mão alcança.",
        "{} {} is inside {} ({}): an appliance's outlet goes beside it or above its top ({} cm), where a hand can reach.",
    ),
    (
        "{} {} vai no piso de um cômodo e está fora de todos.",
        "{} {} goes on a room's floor and is outside all of them.",
    ),
    (
        "{} {} está dentro da parede {}: o ralo vai no piso, fora dela.",
        "{} {} is inside wall {}: a drain goes in the floor, outside it.",
    ),
    (
        "{} {} está no vão de {} ({}): a soleira não leva ralo; ponha-o dentro do cômodo.",
        "{} {} is in the opening of {} ({}): a threshold takes no drain; put it inside the room.",
    ),
    (
        "{} {} está solto no meio do cômodo: vai embutido numa parede, ou no móvel fixo de uma ilha ou bancada.",
        "{} {} is loose in the middle of the room: it is built into a wall, or into the fixed cabinet of an island or counter.",
    ),
    (
        "{} {} fica embaixo de {} ({}): sem acesso para limpar e desentupir.",
        "{} {} is under {} ({}): with no access to clean and unclog it.",
    ),
    (
        "{} {} fica atrás da folha aberta de {} ({}): ponha-o do lado da maçaneta.",
        "{} {} is behind the open leaf of {} ({}): put it on the handle side.",
    ),
    (
        "Nenhum dormitório identificado: nomeie os cômodos (Quarto, Suíte) ou coloque camas.",
        "No bedroom identified: name the rooms (Bedroom, Suite) or place beds.",
    ),
    (
        "Nenhum banheiro com vaso sanitário.",
        "No bathroom with a toilet.",
    ),
    (
        "Morador idoso: barras de apoio junto ao vaso e no box, piso antiderrapante e box sem degrau.",
        "Elderly resident: grab bars by the toilet and in the shower, a non-slip floor and a step-free shower.",
    ),
    (
        "Pia e fogão colados: deixe bancada entre eles para preparo e segurança (a NKBA soma 90 cm de apoio entre os dois).",
        "Sink and stove side by side: leave counter between them for preparation and safety (the NKBA adds up 90 cm of landing space between the two).",
    ),
    (
        "Uma grelha de ventilação permanente: a ventilação de aparelho a gás pede abertura inferior e superior; confira a área útil na edição vigente da norma.",
        "One permanent ventilation grille: a gas appliance's ventilation needs a low and a high opening; check the free area in the current edition of the standard.",
    ),
    (
        "Aparelho a gás em ambiente sem janela nem abertura permanente: a ventilação permanente para o exterior é obrigatória (em São Paulo, Decreto 57.776, 3.M), e aqui é segurança, não conforto.",
        "Gas appliance in a room with no window or permanent opening: permanent ventilation to the outside is mandatory (in São Paulo, Decree 57.776, 3.M), and here it is safety, not comfort.",
    ),
    (
        "Aparelho a gás: janela que fecha não é ventilação permanente. Preveja abertura permanente direta para o exterior (veneziana ou grelha, inferior e superior), como pedem a norma de aparelhos a gás e, em São Paulo, o Decreto 57.776 (3.M).",
        "Gas appliance: a window that closes is not permanent ventilation. Provide a permanent opening straight to the outside (a louvre or grille, low and high), as the gas appliance standard and, in São Paulo, Decree 57.776 (3.M) require.",
    ),
    (
        "Cocção sem coifa: o cozimento gera material particulado e NO₂ dentro de casa, e sem captura eles ficam no ar da sala junto.",
        "Cooking with no hood: cooking releases particulate matter and NO₂ indoors, and without capture they stay in the air of the living room too.",
    ),
    (
        "Código de obras não informado: o círculo livre no piso e as áreas mínimas da cozinha variam por município — informe a cidade para que sejam verificados.",
        "Building code not given: the clear floor circle and the kitchen's minimum areas vary by municipality — give the city so they can be checked.",
    ),
    (
        "Aquecedor a gás no banheiro: ali só é admitido aparelho tipo C (câmara de combustão estanque); do contrário, leve-o para fora, para a área de serviço ventilada.",
        "Gas heater in the bathroom: only a type C appliance (sealed combustion chamber) is allowed there; otherwise, move it out to the ventilated service area.",
    ),
    (
        "Cocção a gás no mesmo ambiente em que se dorme: a norma limita a 8,14 kW, com válvula de segurança em todos os queimadores e coifa com saída para o exterior.",
        "Gas cooking in the same room people sleep in: the standard limits it to 8.14 kW, with a safety valve on every burner and a hood venting outside.",
    ),
    (
        "passagem aos pés da cama",
        "walking past the foot of the bed",
    ),
    ("acesso ao berço", "reaching the crib"),
    (
        "abrir as portas e circular diante do guarda-roupa; portas de correr pedem menos",
        "opening the doors and moving in front of the wardrobe; sliding doors need less",
    ),
    (
        "abrir as portas e circular diante do guarda-roupa",
        "opening the doors and moving in front of the wardrobe",
    ),
    (
        "abrir gavetas e ficar diante delas",
        "opening drawers and standing in front of them",
    ),
    ("giro de cadeira de rodas", "wheelchair turn"),
    (
        "circulação diante de bancada e equipamentos",
        "circulation in front of counters and appliances",
    ),
    (
        "circulação em volta da ilha",
        "circulation around the island",
    ),
    ("área de transferência", "transfer area"),
    ("uso do vaso", "using the toilet"),
    (
        "aproximação frontal ao lavatório",
        "front approach to the basin",
    ),
    ("uso do lavatório", "using the basin"),
    ("entrar e sair do box", "getting in and out of the shower"),
    (
        "uso do tanque e da máquina de lavar",
        "using the laundry sink and the washing machine",
    ),
    (
        "baixar a porta da máquina por inteiro",
        "lowering the machine's door all the way",
    ),
    (
        "abrir a porta da máquina e agachar diante dela para carregar",
        "opening the machine's door and crouching in front of it to load",
    ),
    (
        "cadeira e levantar-se da mesa",
        "a chair and getting up from the desk",
    ),
    (
        "puxar a cadeira e sentar",
        "pulling out the chair and sitting down",
    ),
    ("passar atrás das cadeiras", "walking behind the chairs"),
    (
        "sentar, levantar e circular diante do assento",
        "sitting, getting up and moving in front of the seat",
    ),
    ("transferência da cadeira", "transfer from the wheelchair"),
    ("circulação ao lado da cama", "circulation beside the bed"),
    ("fogão ou cooktop", "stove or cooktop"),
    ("geladeira", "fridge"),
    ("pia", "sink"),
    ("vaso sanitário", "toilet"),
    ("lavatório", "basin"),
    ("fogão", "stove"),
    ("armazenagem", "storage"),
    ("bancada", "counter"),
    ("mantimentos", "provisions"),
    ("lavagem", "washing"),
    ("preparo", "preparation"),
    ("guarda-roupa", "wardrobe"),
    ("box ou banheira", "shower or bathtub"),
    ("sala", "living room"),
    ("sala de jantar", "dining room"),
    ("cozinha", "kitchen"),
    ("área de serviço", "service area"),
    ("escritório", "office"),
    ("circulação", "circulation"),
    ("ambiente", "room"),
    ("pia de cozinha", "kitchen sink"),
    ("chuveiro", "shower"),
    ("banheira", "bathtub"),
    ("máquina de lavar", "washing machine"),
    ("tanque", "laundry sink"),
    ("lava-louças", "dishwasher"),
    ("bidê", "bidet"),
    ("o vaso sanitário", "the toilet"),
    ("o lavatório", "the basin"),
    ("a pia de cozinha", "the kitchen sink"),
    ("o chuveiro", "the shower"),
    ("a banheira", "the bathtub"),
    ("a máquina de lavar", "the washing machine"),
    ("o tanque", "the laundry sink"),
    ("a lava-louças", "the dishwasher"),
    ("o bidê", "the bidet"),
    ("gradil", "railing"),
    ("guarda-corpo de vidro", "glass guard"),
    ("fechamento de vidro da sacada", "balcony glazing"),
    ("cama", "bed"),
    ("{}, {}", "{}, {}"),
    ("{} e {}", "{} and {}"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_renders_the_portuguese_format_would_have() {
        let (free, min) = ("37,5", "40");
        let text = crate::say!("{} cm livres (mínimo {} cm).", free, min);
        assert_eq!(
            text.as_str(),
            format!("{free} cm livres (mínimo {min} cm).")
        );
        assert_eq!(&*text, "37,5 cm livres (mínimo 40 cm).");
        let fixed: Text = "Nenhum banheiro com vaso sanitário.".into();
        assert_eq!(fixed, "Nenhum banheiro com vaso sanitário.");
        assert_eq!(
            fixed.templates(),
            vec!["Nenhum banheiro com vaso sanitário."]
        );
    }

    #[test]
    fn parts_join_with_a_space_in_every_language() {
        let text = crate::say!("A {}.", 1).then(crate::say!("B {}.", 2));
        assert_eq!(text.as_str(), "A 1. B 2.");
        assert_eq!(text.in_language(Language::English), "A 1. B 2.");
    }

    #[test]
    fn holes_are_counted_and_anything_else_is_refused() {
        assert_eq!(holes("a {} b {}"), Some(2));
        assert_eq!(holes("sem nada"), Some(0));
        assert_eq!(holes("{people} moradores"), None);
        assert_eq!(holes("{:.0}"), None);
    }

    /// The string literals right after each `marker` in the part of `file`
    /// above its tests.
    pub(crate) fn literals_after<'a>(file: &'a str, marker: &str) -> Vec<&'a str> {
        let body = file.split("\nmod tests {").next().unwrap_or(file);
        body.match_indices(marker)
            .filter_map(|(at, _)| {
                let rest = body[at + marker.len()..].trim_start();
                let rest = rest.strip_prefix('"')?;
                let mut end = 0;
                let bytes = rest.as_bytes();
                while end < bytes.len()
                    && !(bytes[end] == b'"' && (end == 0 || bytes[end - 1] != b'\\'))
                {
                    end += 1;
                }
                Some(&rest[..end])
            })
            .collect()
    }

    #[test]
    fn every_sentence_a_discipline_says_has_its_english() {
        let files = [
            include_str!("electrical.rs"),
            include_str!("plumbing.rs"),
            include_str!("guard.rs"),
            include_str!("mounting.rs"),
        ];
        let mut said = 0;
        for file in files {
            for marker in ["say!(", "message: "] {
                for template in literals_after(file, marker) {
                    // An escaped quote is how the TV's inches are written.
                    let template = template.replace("\\\"", "\"");
                    said += 1;
                    assert!(english(&template).is_some(), "no English for: {template}");
                }
            }
        }
        assert!(said > 80, "only {said} sentences found: is it reading?");
    }

    #[test]
    fn every_english_row_follows_its_portuguese() {
        let mut seen = std::collections::BTreeSet::new();
        for (pt, en) in ENGLISH {
            assert!(seen.insert(*pt), "twice in the table: {pt}");
            let (a, b) = (holes(pt), holes(en));
            assert!(a.is_some(), "only {{}} in a template: {pt}");
            assert_eq!(a, b, "the same holes on both sides:\n{pt}\n{en}");
            // Punctuation alone is the same in both.
            if pt.chars().any(char::is_alphabetic) {
                assert_ne!(pt, en, "not translated: {pt}");
            }
        }
    }
}
