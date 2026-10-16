"""Generate the message catalogue exclusively from the bundled official XSDs."""
from pathlib import Path
import json
import sys
import xml.etree.ElementTree as ET
root = Path(__file__).resolve().parents[1]
ns = {"xs": "http://www.w3.org/2001/XMLSchema"}
entries = []
for file in sorted((root / "crates/core-lib/xbau/2.7/xsd_dev").glob("xbau-*.xsd")):
    schema = ET.parse(file).getroot()
    for element in schema.findall("xs:element", ns):
        name = element.get("name", "")
        if name[-4:].isdigit():
            title = element.find("xs:annotation/xs:appinfo/title", ns)
            entries.append({"element": name, "namensraum": schema.get("targetNamespace"),
                            "titel": title.text if title is not None else name, "schema": file.name})
path = root / "crates/core-lib/src/xbau/nachrichten.json"
text = json.dumps(entries, ensure_ascii=False, indent=2) + "\n"
if "--check" in sys.argv:
    if path.read_text() != text:
        raise SystemExit("XBau catalogue is stale; run python3 scripts/xbau-katalog.py")
else:
    path.write_text(text)
print(f"XBau catalogue: {len(entries)} official message declarations")
