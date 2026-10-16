#!/usr/bin/env python3
"""Actualiza las listas oficiales C2PA vendorizadas en trust/c2pa/.

Uso: scripts/update-c2pa-trust.py [COMMIT | --list-eligible]

--list-eligible imprime, sin red, los productos elegibles de los ficheros ya
vendorizados (lo usa el test de paridad con Rust).

COMMIT es un SHA (corto o completo) de c2pa-org/conformance-public. Sin
argumento usa el ultimo commit de main. Descarga los tres ficheros desde
raw.githubusercontent.com a ese SHA, comprueba que parsean (JSON / PEM),
reescribe trust/c2pa/SOURCE.lock y escribe en stdout un resumen de los
productos que ganan o pierden la elegibilidad (assurance >= 2).

Determinista: el mismo SHA produce exactamente los mismos bytes. Si los
hashes no cambian, no se toca nada (ni siquiera SOURCE.lock).

Solo libreria estandar. Usa GITHUB_TOKEN si existe (limite de API).
"""
import datetime
import hashlib
import json
import os
import re
import sys
import urllib.request

REPO = "c2pa-org/conformance-public"
FILES = {
    "C2PA-TRUST-LIST.pem": "trust-list/C2PA-TRUST-LIST.pem",
    "C2PA-TSA-TRUST-LIST.pem": "trust-list/C2PA-TSA-TRUST-LIST.pem",
    "conforming-products-list.json": "conforming-products/conforming-products-list.json",
}
# Debe coincidir con MIN_EXTERNAL_ASSURANCE en crates/core/src/external_trust.rs.
MIN_ASSURANCE = 2

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "trust", "c2pa")


def http_get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "delta-sync-c2pa-trust"})
    token = os.environ.get("GITHUB_TOKEN")
    if token and url.startswith("https://api.github.com/"):
        req.add_header("Authorization", "Bearer " + token)
    with urllib.request.urlopen(req, timeout=60) as resp:
        return resp.read()


def resolve(ref):
    data = http_get("https://api.github.com/repos/%s/commits/%s" % (REPO, ref))
    sha = json.loads(data)["sha"]
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        sys.exit("SHA inesperado: %r" % sha)
    return sha


def verify(name, data):
    if name.endswith(".pem"):
        text = data.decode("ascii")
        begins = text.count("-----BEGIN CERTIFICATE-----")
        ends = text.count("-----END CERTIFICATE-----")
        if begins == 0 or begins != ends:
            sys.exit("%s: PEM invalido (%d BEGIN, %d END)" % (name, begins, ends))
        return "%d certificados" % begins
    records = json.loads(data.decode("utf-8"))
    if not isinstance(records, list) or not records:
        sys.exit("%s: se esperaba una lista no vacia" % name)
    for r in records:
        if "status" not in r or "product" not in r or "DN" not in r["product"]:
            sys.exit("%s: registro sin status/product/DN" % name)
    return "%d registros" % len(records)


# Debe coincidir con signal_recognised / signal_matches_capture de Rust.
KNOWN_SIGNALS = {"capturedMedia", "capturedMediaStitched", "nonGenAIDigitalCreation", "digitalCapture"}
CAPTURE_SIGNALS = {"capturedMedia", "digitalCapture"}


def eligible_record(r):
    """Misma regla que TrustStore::eligible_products en Rust (condiciones 2, 3 y 5).

    Se miran inception y transformation. Una senal que el codigo no reconoce
    (ni nombre conocido ni etiqueta con punto, p. ej. c2pa.resized) descarta el
    producto: falla cerrado, igual que Rust.
    """
    p = r["product"]
    level = (p.get("assurance") or {}).get("maxAssuranceLevel") or 0
    ds = p.get("disallowedSignals") or {}
    signals = (ds.get("inception") or []) + (ds.get("transformation") or [])
    return (
        r.get("status") == "conformant"
        and p.get("productType") == "generatorProduct"
        and level >= MIN_ASSURANCE
        and all(s in KNOWN_SIGNALS or "." in s for s in signals)
        and not any(s in CAPTURE_SIGNALS for s in signals)
    )


def eligible(data):
    """Productos (O, CN) elegibles como camara externa."""
    out = {}
    if data is None:
        return out
    for r in json.loads(data.decode("utf-8")):
        if eligible_record(r):
            p = r["product"]
            dn = p["DN"]
            out.setdefault((dn.get("O", ""), dn.get("CN", "")), []).append(p.get("minVersion") or "-")
    return out


def read(path):
    try:
        with open(path, "rb") as f:
            return f.read()
    except FileNotFoundError:
        return None


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "--list-eligible":
        # Sin red: lista el conjunto elegible de los ficheros vendorizados.
        for o, cn in sorted(eligible(read(os.path.join(OUT, "conforming-products-list.json")))):
            print("%s / %s" % (o, cn))
        return
    ref = sys.argv[1] if len(sys.argv) > 1 else "main"
    sha = resolve(ref)
    print("commit: %s" % sha, file=sys.stderr)
    new = {}
    for name, path in FILES.items():
        url = "https://raw.githubusercontent.com/%s/%s/%s" % (REPO, sha, path)
        data = http_get(url)
        print("%s: %s" % (name, verify(name, data)), file=sys.stderr)
        new[name] = data
    old = {name: read(os.path.join(OUT, name)) for name in FILES}
    hashes = {n: hashlib.sha256(d).hexdigest() for n, d in new.items()}
    if all(old[n] is not None and hashlib.sha256(old[n]).hexdigest() == hashes[n] for n in FILES):
        print("Sin cambios en las listas oficiales (commit %s)." % sha[:10])
        return

    os.makedirs(OUT, exist_ok=True)
    for name, data in new.items():
        with open(os.path.join(OUT, name), "wb") as f:
            f.write(data)
    fetched = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    lock = [
        "# Generado por scripts/update-c2pa-trust.py. No editar a mano.",
        'repo = "%s"' % REPO,
        'commit = "%s"' % sha,
        'fetched_at = "%s"' % fetched,
        "",
    ]
    for name, path in FILES.items():
        lock += ["[[file]]", 'name = "%s"' % name, 'source = "%s"' % path, 'sha256 = "%s"' % hashes[name], ""]
    with open(os.path.join(OUT, "SOURCE.lock"), "w", newline="\n") as f:
        f.write("\n".join(lock).rstrip("\n") + "\n")

    a, b = eligible(old["conforming-products-list.json"]), eligible(new["conforming-products-list.json"])
    print("Listas oficiales C2PA actualizadas al commit %s." % sha)
    print("")
    print("Cambios de ficheros:")
    for n in FILES:
        state = "nuevo" if old[n] is None else ("cambiado" if old[n] != new[n] else "igual")
        print("- %s: %s" % (n, state))
    print("")
    print("Productos con assurance >= %d elegibles como camara externa:" % MIN_ASSURANCE)
    gained = sorted(set(b) - set(a))
    lost = sorted(set(a) - set(b))
    for k in gained:
        print("- GANA: %s / %s (minVersion: %s)" % (k[0], k[1], ", ".join(sorted(b[k]))))
    for k in lost:
        print("- PIERDE: %s / %s" % (k[0], k[1]))
    for k in sorted(set(a) & set(b)):
        if sorted(a[k]) != sorted(b[k]):
            print("- CAMBIA versiones: %s / %s: %s -> %s" % (k[0], k[1], ", ".join(sorted(a[k])), ", ".join(sorted(b[k]))))
    if not gained and not lost:
        print("- (sin altas ni bajas)")
    print("")
    print("Conjunto elegible actual (%d): %s" % (len(b), "; ".join("%s / %s" % k for k in sorted(b))))


if __name__ == "__main__":
    main()
