"""Build a per-user x64 MSI using Python 3.12's Windows Installer API.

Does not install anything, execute payloads, or modify security settings.
Usage: python build_msi.py --payload PORTABLE_DIRECTORY --output FILE.msi
"""
import argparse
import hashlib
import json
from pathlib import Path
import tempfile
import uuid
import msilib as m
from msilib import schema

NAMESPACE = uuid.UUID('ed8e4be9-b39c-4344-9d21-625059541b8a')


def guid(name):
    return '{' + str(uuid.uuid5(NAMESPACE, name)).upper() + '}'


def rows(db, table):
    view = db.OpenView('SELECT * FROM `' + table + '`')
    view.Execute(None)
    result = []
    while (record := view.Fetch()) is not None:
        result.append([record.GetString(i) for i in range(1, record.GetFieldCount() + 1)])
    view.Close()
    return result


def build(payload, output):
    files = [('AppExe', 'FITIG-SEO-Spider.exe', 'FITIG.EXE'),
             ('WebViewLoader', 'WebView2Loader.dll', 'WV2LOAD.DLL')]
    for _, name, _ in files:
        if not (payload / name).is_file():
            raise ValueError('Missing payload: ' + name)
    if output.exists():
        raise FileExistsError('Refusing to overwrite an existing MSI: ' + str(output))
    output.parent.mkdir(parents=True, exist_ok=True)
    db = m.init_database(str(output), schema, 'FITIG SEO Spider',
                         guid('product-0.1.6-x64'), '0.1.6', 'FITIG')
    summary = db.GetSummaryInformation(1)
    summary.SetProperty(m.PID_WORDCOUNT, 10)  # Compressed, per-user; no elevation.
    summary.Persist()
    del summary
    m.add_data(db, 'Property', [('UpgradeCode', guid('upgrade-x64')),
        ('ARPNOMODIFY', '1'), ('ARPNOREPAIR', '1'),
        ('INSTALLLEVEL', '1')])
    m.add_data(db, 'Directory', [('TARGETDIR', None, 'SourceDir'),
        ('LocalAppDataFolder', 'TARGETDIR', '.'),
        ('ProgramsDir', 'LocalAppDataFolder', 'Programs'),
        ('INSTALLDIR', 'ProgramsDir', 'FITIG|FITIG SEO Spider'),
        ('ProgramMenuFolder', 'TARGETDIR', '.'),
        ('MenuDir', 'ProgramMenuFolder', 'FITIG|FITIG SEO Spider')])
    m.add_data(db, 'Feature', [('Main', None, 'FITIG SEO Spider',
        'Desktop crawler', 1, 1, 'INSTALLDIR', 0)])
    m.add_data(db, 'LaunchCondition', [('VersionNT64', '64-bit Windows is required.')])
    for index, (key, name, short) in enumerate(files, 1):
        component = key + 'Component'
        m.add_data(db, 'Component', [(component, guid(component), 'INSTALLDIR',
                                   256, None, key)])
        m.add_data(db, 'FeatureComponents', [('Main', component)])
        m.add_data(db, 'File', [(key, component, short + '|' + name,
            (payload / name).stat().st_size, None, None, 512, index)])
    m.add_data(db, 'Component', [('MenuComponent', guid('MenuComponent'),
        'MenuDir', 260, None, 'MenuRegistry')])
    m.add_data(db, 'FeatureComponents', [('Main', 'MenuComponent')])
    m.add_data(db, 'Registry', [('MenuRegistry', 1,
        r'Software\FITIG\SEO Spider', 'Installed', '#1', 'MenuComponent')])
    m.add_data(db, 'Shortcut', [('AppShortcut', 'MenuDir',
        'FITIG|FITIG SEO Spider', 'MenuComponent', '[#AppExe]', None,
        'FITIG SEO Spider', None, None, None, 1, 'INSTALLDIR')])
    m.add_data(db, 'RemoveFile', [('MenuFolder', 'MenuComponent', None, 'MenuDir', 2)])
    actions = [('LaunchConditions', 100), ('CostInitialize', 800),
        ('FileCost', 900), ('CostFinalize', 1000), ('InstallValidate', 1400),
        ('InstallInitialize', 1500), ('ProcessComponents', 1600),
        ('UnpublishFeatures', 1800), ('RemoveRegistryValues', 2600),
        ('RemoveShortcuts', 3200), ('RemoveFiles', 3500), ('RemoveFolders', 3600),
        ('InstallFiles', 4000), ('CreateShortcuts', 4500),
        ('WriteRegistryValues', 5000), ('RegisterUser', 6000),
        ('RegisterProduct', 6100), ('PublishFeatures', 6300),
        ('PublishProduct', 6400), ('InstallFinalize', 6600)]
    m.add_data(db, 'InstallExecuteSequence', [(a, None, n) for a, n in actions])
    m.add_data(db, 'InstallUISequence', [(a, None, n) for a, n in
        [('LaunchConditions', 100), ('CostInitialize', 800), ('FileCost', 900),
         ('CostFinalize', 1000), ('ExecuteAction', 1300)]])
    # Native cabinet creation, no external compiler or payload execution.
    with tempfile.TemporaryDirectory(prefix='fitig-msi-', dir=output.parent) as temporary:
        cab = Path(temporary) / 'payload.cab'
        m.FCICreate(str(cab), [(str(payload / name), key) for key, name, _ in files])
        import subprocess
        import os
        expanded = Path(temporary) / 'expanded'
        expanded.mkdir()
        subprocess.run([str(Path(os.environ['SystemRoot']) / 'System32' / 'expand.exe'),
            str(cab), '-F:*', str(expanded)], check=True, capture_output=True)
        for key, name, _ in files:
            assert hashlib.sha256((expanded / key).read_bytes()).digest() == \
                hashlib.sha256((payload / name).read_bytes()).digest()
        m.add_stream(db, 'payload.cab', str(cab))
        m.add_data(db, 'Media', [(1, len(files), None, '#payload.cab', None, None)])
    db.Commit()
    del db
    check = m.OpenDatabase(str(output), m.MSIDBOPEN_READONLY)
    assert len(rows(check, 'File')) == 2
    assert len(rows(check, 'Component')) == 3
    assert rows(check, 'CustomAction') == []
    properties = dict(rows(check, 'Property'))
    assert 'ALLUSERS' not in properties
    assert properties['ProductVersion'] == '0.1.6'
    evidence = {'version': '0.1.6', 'package': str(output),
        'sha256': hashlib.sha256(output.read_bytes()).hexdigest(),
        'validation': 'Database tables inspected; CAB extracted and both payload hashes match; installation not executed',
        'files': {name: hashlib.sha256((payload / name).read_bytes()).hexdigest()
                  for _, name, _ in files},
        'tables': {table: rows(check, table) for table in
            ['File', 'Directory', 'Component', 'Shortcut', 'Media', 'Property']}}
    output.with_suffix('.validation.json').write_text(
        json.dumps(evidence, indent=2), encoding='utf-8')
    print(json.dumps({'package': str(output), 'sha256': evidence['sha256']}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--payload', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    build(args.payload.resolve(), args.output.resolve())
