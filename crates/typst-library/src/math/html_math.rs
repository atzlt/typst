use ecow::eco_format;
use typst_library::{
    diag::SourceResult,
    engine::Engine,
    foundations::{Content, NativeElement, Packed, SequenceElem, StyleChain, SymbolElem},
    html::{math, HtmlAttr, HtmlElem},
    math::{
        AccentElem, AttachElem, CancelElem, FracElem, LrElem, OpElem, PrimesElem
    },
    text::TextElem,
};
use unicode_math_class::MathClass;

pub fn html_show_equation(
    elem: &Content,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    if let Some(sequence) = elem.to_packed::<SequenceElem>() {
        let c: SourceResult<Vec<_>> = sequence
            .children
            .iter()
            .map(|c| html_show_equation(c, engine, styles))
            .collect();
        Ok(HtmlElem::new(math::mrow)
            .with_body(Some(Content::sequence(c?)))
            .pack()
            .spanned(elem.span()))
    } else if let Some(text) = elem.to_packed::<TextElem>() {
        let is_num = text.text.to_string().parse::<f32>().is_ok();
        Ok(HtmlElem::new(if is_num { math::mn } else { math::mtext })
            .with_body(Some(elem.clone()))
            .pack()
            .spanned(elem.span()))
    } else if let Some(elem) = elem.to_packed::<LrElem>() {
        show_lr(elem, engine, styles)
    } else if let Some(elem) = elem.to_packed::<FracElem>() {
        show_frac(elem, engine, styles)
    } else if let Some(elem) = elem.to_packed::<AttachElem>() {
        show_attach(elem, engine, styles)
    } else if let Some(elem) = elem.to_packed::<CancelElem>() {
        let notation = if elem.cross(styles) {
            "updiagonalstrike downdiagonalstrike"
        } else if elem.inverted(styles) {
            "downdiagonalstrike"
        } else {
            "updiagonalstrike"
        };
        let tag = HtmlElem::new(math::menclose)
            .with_attr(HtmlAttr::constant("notation"), notation);
        let body = elem.body.clone();
        Ok(tag.with_body(Some(body)).pack().spanned(elem.span()))
    } else if let Some(elem) = elem.to_packed::<AccentElem>() {
        let accent = TextElem::packed(eco_format!(" {}", elem.accent.0));
        let accent = HtmlElem::new(math::mo).with_body(Some(accent)).pack();
        let body = Content::sequence([elem.base.clone(), accent]);
        Ok(HtmlElem::new(math::mover)
            .with_body(Some(body))
            .pack()
            .spanned(elem.span()))
    } else if let Some(elem) = elem.to_packed::<SymbolElem>() {
        let is_op = unicode_math_class::class(elem.text) != Some(MathClass::Alphabetic);
        Ok(HtmlElem::new(if is_op { math::mo } else { math::mi })
            .with_body(Some(TextElem::new(elem.text.into()).into()))
            .pack()
            .spanned(elem.span()))
    } else if let Some(elem) = elem.to_packed::<OpElem>() {
        Ok(HtmlElem::new(math::mo)
            .with_body(Some(elem.text.clone()))
            .pack()
            .spanned(elem.span()))
    } else {
        Ok(elem.clone())
    }
}

fn show_lr(
    elem: &Packed<LrElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    /*
    if let Some(seq) = body.to_packed::<SequenceElem>() {
        let children = &seq.children;
        match &children[..] {
            [l, mid @ .., r] => todo!(),
            _ => todo!(),
        }
    }
    dbg!(&body);
    */
    html_show_equation(&elem.body, engine, styles)
}

fn show_frac(
    elem: &Packed<FracElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let num = html_show_equation(&elem.num, engine, styles)?;
    dbg!(&elem.denom);
    let denom = html_show_equation(&elem.denom, engine, styles)?;
    dbg!(&denom);
    let body = Content::sequence([num, denom]);
    Ok(HtmlElem::new(math::mfrac)
        .with_body(Some(body))
        .pack()
        .spanned(elem.span()))
}

fn show_attach(
    elem: &Packed<AttachElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let merged = elem.merge_base();
    let elem = merged.as_ref().unwrap_or(elem);

    let base = elem.base.clone();
    let sup_style_chain = styles;
    let tl = elem.tl(sup_style_chain);
    let tr = elem.tr(sup_style_chain);
    let primed = tr.as_ref().is_some_and(|content| content.is::<PrimesElem>());
    let t = elem.t(sup_style_chain);

    let sub_style_chain = styles;
    let bl = elem.bl(sub_style_chain);
    let br = elem.br(sub_style_chain);
    let b = elem.b(sub_style_chain);

    let mut limits = false;
    if let Some(base) = base.to_packed::<OpElem>() {
        limits = base.limits(styles);
    }
    let limits = limits;

    let (t, tr) = match (t, tr) {
        (Some(t), Some(tr)) if primed && !limits => (None, Some(tr + t)),
        (Some(t), None) if !limits => (None, Some(t)),
        (t, tr) => (t, tr),
    };
    let (b, br) = if limits || br.is_some() { (b, br) } else { (None, b) };

    let none = || HtmlElem::new(math::mrow).pack();
    let br = br
        .map(|c| html_show_equation(&c, engine, styles))
        .transpose()?
        .unwrap_or_else(none);
    let tr = tr
        .map(|c| html_show_equation(&c, engine, styles))
        .transpose()?
        .unwrap_or_else(none);
    let bl = bl
        .map(|c| html_show_equation(&c, engine, styles))
        .transpose()?
        .unwrap_or_else(none);
    let tl = tl
        .map(|c| html_show_equation(&c, engine, styles))
        .transpose()?
        .unwrap_or_else(none);
    let b = b.map(|c| html_show_equation(&c, engine, styles)).transpose()?;
    let t = t.map(|c| html_show_equation(&c, engine, styles)).transpose()?;
    let prescripts = HtmlElem::new(math::mprescripts).pack();

    let base = match (b, t) {
        (Some(b), Some(t)) => HtmlElem::new(math::munderover)
            .with_body(Some(Content::sequence([base, b, t])))
            .pack(),
        (None, None) => base,
        _ => todo!(),
    };
    let base = html_show_equation(&base, engine, styles)?;

    let body = Content::sequence([base, br, tr, prescripts, bl, tl]);
    Ok(HtmlElem::new(math::mmultiscripts)
        .with_body(Some(body))
        .pack()
        .spanned(elem.span()))
}
