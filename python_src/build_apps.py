import os
import subprocess
import sys
import platform

def install_build_deps():
    print("Checking and installing build dependencies...")
    subprocess.check_call([
        sys.executable, "-m", "pip", "install", 
        "pyinstaller", "mss", "PyQt6",
        "--trusted-host", "pypi.org",
        "--trusted-host", "pypi.python.org",
        "--trusted-host", "files.pythonhosted.org"
    ])

def build():
    install_build_deps()
    
    # Create a custom spec file to filter out large unnecessary binaries
    spec_content = f"""# -*- mode: python ; coding: utf-8 -*-

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[],
    hiddenimports=[],
    hookspath=[],
    hooksconfig={{}},
    runtime_hooks=[],
    excludes={['PyQt6.QtWebEngineCore', 'PyQt6.QtWebEngineWidgets', 'PyQt6.QtQml', 'PyQt6.QtQuick', 'PyQt6.QtSql', 'PyQt6.QtTest', 'PyQt6.QtXml', 'PyQt6.QtNetwork', 'PyQt6.QtBluetooth', 'PyQt6.QtNfc', 'PyQt6.QtPositioning', 'PyQt6.QtMultimedia', 'PyQt6.QtMultimediaWidgets', 'PyQt6.QtSensors', 'PyQt6.QtSerialPort', 'PyQt6.QtWebChannel', 'PyQt6.QtWebSockets', 'PyQt6.QtDesigner', 'PyQt6.QtHelp', 'PyQt6.QtDBus', 'PyQt6.QtOpenGL', 'PyQt6.QtOpenGLWidgets', 'PyQt6.QtPdf', 'PyQt6.QtPdfWidgets', 'PyQt6.QtPrintSupport', 'PyQt6.QtQuick3D', 'PyQt6.QtQuickWidgets', 'PyQt6.QtRemoteObjects', 'PyQt6.QtSpatialAudio', 'PyQt6.QtStateMachine', 'PyQt6.QtSvg', 'PyQt6.QtSvgWidgets', 'PyQt6.QtTextToSpeech', 'tkinter', 'unittest', 'pydoc', 'pdb', 'sqlite3', 'test', 'distutils']},
    noarchive=False,
    optimize=2,
)

# Filter out redundant ICU data and other large binaries
excluded_binaries = [
    'libicudata.so.78', 'libicui18n.so.78', 'libicuuc.so.78', 
    'libQt6Pdf', 'libQt6Network',
    'libgtk-3.so.0', 'libgdk-3.so.0'
]
a.binaries = [x for x in a.binaries if not any(excl in x[0] for excl in excluded_binaries)]

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='CircleToSearch',
    debug=False,
    bootloader_ignore_signals=False,
    strip=True,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
"""
    with open("CircleToSearch.spec", "w") as f:
        f.write(spec_content)

    # Run PyInstaller using the spec file
    cmd = [sys.executable, "-m", "PyInstaller", "--clean", "CircleToSearch.spec"]

    print(f"Running build command: {' '.join(cmd)}")
    
    try:
        subprocess.check_call(cmd)
        print("\n" + "="*50)
        print("BUILD SUCCESSFUL!")
        output_folder = "dist"
        if platform.system() == "Windows":
            print(f"Your portable EXE is in: {os.path.abspath(os.path.join(output_folder, 'CircleToSearch.exe'))}")
        else:
            print(f"Your portable Linux binary is in: {os.path.abspath(os.path.join(output_folder, 'CircleToSearch'))}")
            print("\nTo turn this into a formal AppImage, you can use 'appimagetool' on the dist folder.")
        print("="*50)
    except subprocess.CalledProcessError as e:
        print(f"Build failed with error: {e}")

if __name__ == "__main__":
    build()
