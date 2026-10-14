"""PDF regression entry point after migration to the project-first form workflow."""
from pathlib import Path
import subprocess,sys
root=Path(__file__).resolve().parent
for suite in ['formularbibliothek.py','klare-projektstruktur.py']:
 subprocess.run([sys.executable,str(root/suite)],check=True)
