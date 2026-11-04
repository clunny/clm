//! `scan` on Python packages.

mod common;

use common::{TAIL, lines, names, repo};

#[test]
fn scenario_names_an_init_exposes() {
    // req:check.unreferenced/python-packages@3f15962f
    let dir = repo(&[
        (
            "packages/core/src/acme_core/__init__.py",
            "from .parse import parse\n\
             from .parse import render as show\n\
             from ._impl import helper\n\
             from . import sub\n\
             import os\n\
             def top(): ...\n\
             _private = 1\n\
             VERSION = \"1\"\n",
        ),
        (
            "packages/core/src/acme_core/parse.py",
            "def parse(): ...\n\n\ndef render(): ...\ndef unused_in_parse(): ...\n",
        ),
        (
            "packages/core/src/acme_core/_impl.py",
            "def helper(): ...\n",
        ),
        (
            "packages/core/src/acme_core/sub/__init__.py",
            "def g(): ...\n",
        ),
    ]);
    let at = |path: &str, line: usize, name: &str| {
        format!("packages/core/src/acme_core/{path}:{line}: acme_core::{name} {TAIL}")
    };
    assert_eq!(
        lines(dir.path(), &[]),
        [
            at("__init__.py", 6, "top"),
            at("__init__.py", 8, "VERSION"),
            at("_impl.py", 1, "helper"),
            at("parse.py", 1, "parse"),
            at("parse.py", 4, "show"),
        ]
    );
}

#[test]
fn scenario_all_and_star_imports() {
    // req:check.unreferenced/python-packages@3f15962f
    let dir = repo(&[
        (
            "acme_core/__init__.py",
            "from .a import *\nfrom .b import x, y\n__all__ = [\"x\", \"z\"]\n__all__ += [\"w\"]\n",
        ),
        (
            "acme_core/a.py",
            "__all__ = [\"z\", \"zz\"]\ndef z(): ...\ndef zz(): ...\ndef hidden(): ...\n",
        ),
        ("acme_core/b.py", "\ndef x(): ...\ndef y(): ...\n"),
    ]);
    assert_eq!(
        lines(dir.path(), &[]),
        [
            format!("acme_core/a.py:2: acme_core::z {TAIL}"),
            format!("acme_core/b.py:2: acme_core::x {TAIL}"),
        ]
    );
}

#[test]
fn scenario_packages_scripts_and_test_files() {
    // req:check.unreferenced/python-packages@3f15962f
    let dir = repo(&[
        ("src/acme_core/__init__.py", "def f(): ...\ndef k(): ...\n"),
        ("src/acme_core/tests/test_f.py", "f()\n"),
        ("src/acme_core/conftest.py", "def fixture(): ...\n"),
        ("scripts/run.py", "def h(): ...\n"),
        ("tests/__init__.py", "def in_tests(): ...\n"),
        // Below the package, with a directory without `__init__.py` between: still its own.
        ("src/acme_core/plugins/x/__init__.py", "def in_x(): ...\n"),
    ]);
    assert_eq!(names(dir.path()), ["acme_core::k"]);
}

#[test]
fn bindings_through_compound_statements_targets_and_absolute_imports() {
    // req:check.unreferenced/python-packages@3f15962f
    let dir = repo(&[
        (
            "p/__init__.py",
            "try:\n    from .fast import speed\nexcept ImportError:\n    speed = None\n\
             if True:\n    a = b = 1\n    c, (d, e) = 1, (2, 3)\n\
             n: int = 1\n\
             from p.deep.mod import far\n\
             from os import path\n\
             import p.deep as deep\n\
             @decorated\nclass K: ...\n\
             def outer():\n    inner = 1\n",
        ),
        ("p/fast.py", "def speed(): ...\n"),
        ("p/deep/__init__.py", ""),
        ("p/deep/mod.py", "far = 1\n"),
    ]);
    assert_eq!(
        names(dir.path()),
        [
            "p::K", "p::a", "p::b", "p::c", "p::d", "p::e", "p::far", "p::n", "p::outer",
            "p::speed",
        ]
    );
    let speed: Vec<_> = lines(dir.path(), &[])
        .into_iter()
        .filter(|l| l.contains("p::speed "))
        .collect();
    assert_eq!(speed, [format!("p/fast.py:1: p::speed {TAIL}")]);
}

#[test]
fn star_import_cycles_end() {
    // req:check.unreferenced/python-packages@3f15962f
    let dir = repo(&[
        ("p/__init__.py", "from .a import *\n"),
        ("p/a.py", "from .b import *\ndef fa(): ...\n"),
        ("p/b.py", "from .a import *\ndef fb(): ...\n"),
    ]);
    assert_eq!(names(dir.path()), ["p::fa", "p::fb"]);
}

#[test]
fn broken_pyproject_is_an_error_naming_it() {
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[("svc/pyproject.toml", "[project\n")]);
    let err = clm_unreferenced::scan(dir.path(), &[], &[]).unwrap_err();
    assert!(err.starts_with("svc/pyproject.toml: "), "{err}");
    assert!(!err.contains('\n'), "{err}");
}

#[test]
fn scenario_used_from_another_package_a_test_an_entry_point_and_a_notebook() {
    // req:check.unreferenced/python-uses@4c45195c
    let dir = repo(&[
        (
            "packages/core/src/acme_core/__init__.py",
            "def parse(): ...\ndef render(): ...\ndef main(): ...\nclass Cfg: ...\ndef stale(): ...\n",
        ),
        ("packages/core/src/acme_core/other.py", "stale()\n"),
        ("packages/api/src/acme_api/__init__.py", ""),
        (
            "packages/api/src/acme_api/app.py",
            "from acme_core import parse\n# stale()\n",
        ),
        ("packages/core/tests/test_render.py", "render()\n"),
        (
            "packages/core/pyproject.toml",
            "[project.scripts]\nacme = \"acme_core.cli:main\"\n",
        ),
        (
            "notebooks/demo.ipynb",
            r#"{"cells": [{"source": ["from acme_core import Cfg\n"]}]}"#,
        ),
    ]);
    assert_eq!(names(dir.path()), ["acme_core::stale"]);
}

#[test]
fn every_entry_point_table_counts() {
    // req:check.unreferenced/python-uses@4c45195c
    let dir = repo(&[
        (
            "p/__init__.py",
            "def gui(): ...\ndef plug(): ...\ndef poetry(): ...\ndef pplug(): ...\n\
             def docstring(): ...\ndef idle(): ...\n",
        ),
        (
            "pyproject.toml",
            "[project.gui-scripts]\ng = \"p:gui\"\n\
             [project.entry-points.\"acme.plugins\"]\nx = \"p:plug\"\n\
             [tool.poetry.scripts]\npo = \"p:poetry\"\n\
             [tool.poetry.plugins.\"grp\"]\npp = \"p:pplug\"\n\
             [project]\nname = \"idle\"\n",
        ),
        ("app.py", "\"\"\"Calls p.docstring.\"\"\"\n"),
    ]);
    // `[project] name` isn't an entry point, so `idle` is still reported.
    assert_eq!(names(dir.path()), ["p::idle"]);
}

#[test]
fn scenario_types_reached_through_used_items() {
    // req:check.unreferenced/python-uses@4c45195c
    let dir = repo(&[
        (
            "acme_core/__init__.py",
            "from ._impl import load, Item, Base, save, Options, Lonely, lonely\n",
        ),
        (
            "acme_core/_impl.py",
            "def load() -> Loaded: ...\n\
             class Loaded:\n    items: list[Item]\n\
             class Item(Base): ...\n\
             class Base: ...\n\
             def save(cfg: \"Options\") -> None: ...\n\
             class Options: ...\n\
             class Lonely: ...\n\
             def lonely() -> Lonely: ...\n",
        ),
        ("other/__init__.py", ""),
        ("other/use.py", "load()\nsave(...)\n"),
    ]);
    assert_eq!(
        names(dir.path()),
        ["acme_core::Lonely", "acme_core::lonely"]
    );
}

#[test]
fn relative_imports_stay_within_the_package() {
    // req:check.unreferenced/python-packages@3f15962f
    let dir = repo(&[
        (
            "src/a/__init__.py",
            "from .sub.inner import up\nfrom ..b.m import other\n",
        ),
        ("src/a/sub/__init__.py", ""),
        ("src/a/sub/inner.py", "from ..base import up\n"),
        ("src/a/base.py", "def up(): ...\n"),
        ("src/b/__init__.py", ""),
        ("src/b/m.py", "def other(): ...\n"),
    ]);
    // `a` lists `up` from `..base`, one directory up within it. `other` is `b`'s, and `a` can
    // name it only through a module outside itself, which `a` never lists.
    assert_eq!(
        lines(dir.path(), &[]),
        [format!("src/a/base.py:1: a::up {TAIL}")]
    );
}

#[test]
fn all_forms_and_star_apis_by_their_own_rules() {
    // req:check.unreferenced/python-packages@3f15962f
    let dir = repo(&[
        (
            "p/__init__.py",
            "from .a import *\n__all__: list[str] = (\"t\",)\n\
             __all__ += [\"_u\", \"pub\"]\ndef t(): ...\ndef _u(): ...\n",
        ),
        ("p/a.py", "__all__ = [\"pub\"]\ndef pub(): ...\n"),
        ("q/__init__.py", "from .a import *\n"),
        ("q/a.py", "__all__ = [\"_x\"]\ndef _x(): ...\n"),
        ("r/__init__.py", "from .b import *\n"),
        (
            "r/b.py",
            "__all__ = [\"bb\"] + [\"cc\"]\ndef bb(): ...\ndef cc(): ...\n",
        ),
        ("s/__init__.py", "def f(): ...\n"),
        ("s/__init__.pyi", "def stub_only(): ...\n"),
    ]);
    // A non-literal `__all__` is read as none, so `r` lists what `b.py` binds.
    assert_eq!(
        names(dir.path()),
        ["p::_u", "p::pub", "p::t", "q::_x", "r::bb", "r::cc", "s::f"]
    );
}

#[test]
fn renamed_imports_and_variable_annotations_carry() {
    // req:check.unreferenced/python-uses@4c45195c
    let dir = repo(&[
        (
            "p/__init__.py",
            "from .impl import render as show, Shown, Conf, Idle\nsettings: Conf = Conf()\n",
        ),
        (
            "p/impl.py",
            "def render() -> Shown: ...\nclass Shown: ...\nclass Conf: ...\nclass Idle: ...\n",
        ),
        ("app.py", "show()\nsettings\n"),
    ]);
    assert_eq!(names(dir.path()), ["p::Idle"]);
}

#[test]
fn pyi_files_count_and_untracked_files_do_not() {
    // req:check.unreferenced/python-uses@4c45195c
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        (
            "p/__init__.py",
            "def in_stub(): ...\ndef in_untracked(): ...\n",
        ),
        ("typings/q.pyi", "from p import in_stub\n"),
    ]);
    std::fs::write(dir.path().join("loose.py"), "in_untracked()\n").unwrap();
    assert_eq!(names(dir.path()), ["p::in_untracked"]);
}
