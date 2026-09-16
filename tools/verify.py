"""Execute all local release gates, record exact commands and environment."""
import hashlib,json,os,platform,subprocess,tempfile
from pathlib import Path

commands=[['cargo','fmt','--check'],['cargo','fmt','--manifest-path','consumer/Cargo.toml','--check'],['cargo','clippy','--offline','--all-targets','--','-D','warnings'],['cargo','test','--release','--offline','--','--include-ignored'],['cargo','build','--release','--offline','--manifest-path','consumer/Cargo.toml'],['env','-i','PATH=/nonexistent','consumer/target/release/native-consumer-check',str(Path('assets/en_core_web_md-3.8.0').resolve())],['cargo','package','--allow-dirty','--offline']]
results=[]
for cmd in commands:
    p=subprocess.run(cmd,text=True,capture_output=True)
    results.append({'command':cmd,'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
    if p.returncode:
        Path('reports/verification-failure.json').write_text(json.dumps(results,indent=2));print(p.stdout,p.stderr);raise SystemExit(p.returncode)
    print('PASS',' '.join(cmd))
# Re-export independently and require byte-identical assets, including provenance.
with tempfile.TemporaryDirectory(prefix='native-spacy-export-') as tmp:
    subprocess.run(['.venv/bin/python','tools/export.py','--out',tmp],check=True)
    root=Path('assets/en_core_web_md-3.8.0')
    digests={}
    for p in sorted(root.iterdir()):
        if p.is_file():
            digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
            assert digest(p)==digest(Path(tmp)/p.name),f'non-reproducible export: {p.name}'
            digests[p.name]=digest(p)
manifest=json.loads(Path('assets/en_core_web_md-3.8.0/manifest.json').read_text())
report={'platform':platform.platform(),'machine':platform.machine(),'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),'python':platform.python_version(),'reference_versions':manifest['versions'],'model':manifest['model']+' '+manifest['model_version'],'commands':results,'reexport_sha256':digests,'fixture_sha256':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(Path('fixtures').glob('*.json'))},'note':'Local validation environment, not a performance benchmark.'}
Path('reports/verification.json').write_text(json.dumps(report,indent=2))
print('PASS byte-identical official re-export')
