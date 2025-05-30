pub mod error;

use std::fmt;

use jaq_core::{
    Compiler, Ctx, Native, RcIter,
    load::{Arena, File, Loader, import},
};
use jaq_json::Val;
use serde_json::{Number, Value};

use self::error::{Error, FileReports, Result, compile_errors, load_errors};

type Filter = ::jaq_core::Filter<Native<Val>>;

#[derive(Copy, Clone, Debug, Default)]
pub struct Settings {
    pub null_input: bool,
    pub raw_input: bool,
    pub slurp: bool,
}

pub fn run(filter: &str, input: &str, settings: &Settings) -> Result<Value> {
    let mut vals = vec![];
    process(filter, input, settings, |val| vals.push(val))?;

    match &vals[..] {
        [] => Ok(Value::Null),
        [val] => convert_value(val),
        [_, ..] => vals
            .iter()
            .map(convert_value)
            .collect::<Result<_>>()
            .map(Value::Array),
    }
}

fn convert_value(val: &Val) -> Result<Value> {
    match val {
        Val::Null => Ok(Value::Null),
        Val::Bool(v) => Ok(Value::Bool(*v)),
        Val::Int(v) => Number::from_i128(*v as _)
            .map(Value::Number)
            .ok_or(Error::InvalidNumber),
        Val::Float(v) => Number::from_f64(*v)
            .map(Value::Number)
            .ok_or(Error::InvalidNumber),
        Val::Num(v) => v.parse().map(Value::Number).map_err(Error::Json),
        Val::Str(v) => Ok(Value::String((**v).clone())),
        Val::Arr(list) => list
            .iter()
            .map(convert_value)
            .collect::<Result<_>>()
            .map(Value::Array),
        Val::Obj(map) => map
            .iter()
            .map(|(key, val)| convert_value(val).map(|val| ((**key).clone(), val)))
            .collect::<Result<_>>()
            .map(Value::Object),
    }
}

fn process(filter: &str, input: &str, settings: &Settings, mut f: impl FnMut(Val)) -> Result<()> {
    let (_vals, filter) = parse::<&str>(filter, &[]).map_err(Error::Report)?;

    let inputs = read_str(settings, input);

    let inputs = Box::new(inputs) as Box<dyn Iterator<Item = Result<_, _>>>;
    let null = Box::new(core::iter::once(Ok(Val::Null))) as Box<dyn Iterator<Item = _>>;

    let inputs = RcIter::new(inputs);
    let null = RcIter::new(null);

    for x in if settings.null_input { &null } else { &inputs } {
        let x = x.map_err(Error::Hifijson)?;
        for y in filter.run((Ctx::new([], &inputs), x)) {
            f(y.map_err(Error::Jaq)?);
        }
    }
    Ok(())
}

fn parse<V>(code: &str, vars: &[V]) -> Result<(Vec<Val>, Filter), Vec<FileReports>>
where
    V: fmt::Display,
{
    let arena = Arena::default();
    let loader = Loader::new(::jaq_std::defs().chain(::jaq_json::defs()));
    let modules = loader
        .load(&arena, File { path: (), code })
        .map_err(load_errors)?;
    import(&modules, |_path| Err("file loading not supported".into())).map_err(load_errors)?;

    let vals = Vec::new();

    let vars: Vec<_> = vars.iter().map(|v| format!("${v}")).collect();
    let compiler = Compiler::default()
        .with_funs(::jaq_std::funs().chain(::jaq_json::funs()))
        .with_global_vars(vars.iter().map(|v| v.as_str()));
    let filter = compiler.compile(modules).map_err(compile_errors)?;
    Ok((vals, filter))
}

fn read_str<'a>(
    settings: &Settings,
    input: &'a str,
) -> Box<dyn Iterator<Item = Result<Val, String>> + 'a> {
    if settings.raw_input {
        Box::new(raw_input(settings.slurp, input).map(|s| Ok(Val::from(s.to_owned()))))
    } else {
        let vals = json_slice(input.as_bytes());
        Box::new(collect_if(settings.slurp, vals))
    }
}

fn raw_input(slurp: bool, input: &str) -> impl Iterator<Item = &str> {
    if slurp {
        Box::new(core::iter::once(input))
    } else {
        Box::new(input.lines()) as Box<dyn Iterator<Item = _>>
    }
}

fn json_slice(slice: &[u8]) -> impl Iterator<Item = Result<Val, String>> + '_ {
    let mut lexer = ::hifijson::SliceLexer::new(slice);
    core::iter::from_fn(move || {
        use ::hifijson::token::Lex;
        Some(Val::parse(lexer.ws_token()?, &mut lexer).map_err(|e| e.to_string()))
    })
}

fn collect_if<'a, T: 'a + FromIterator<T>, E: 'a>(
    slurp: bool,
    iter: impl Iterator<Item = Result<T, E>> + 'a,
) -> Box<dyn Iterator<Item = Result<T, E>> + 'a> {
    if slurp {
        Box::new(core::iter::once(iter.collect()))
    } else {
        Box::new(iter)
    }
}
