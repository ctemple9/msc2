#!/usr/bin/env python3
"""Refresh the additive storage DTO schemas from their small wire-only Rust file."""
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[2]
path = root / "docs/msc2/api-contract/openapi.json"
original = path.read_text()
doc = json.loads(original)
schemas = doc["components"]["schemas"]
source = (root / "crates/msc-api/src/dto/storage.rs").read_text()

def camel(name):
    first, *rest = name.split("_")
    return first + "".join(word.title() for word in rest)

def shape(kind):
    if kind.startswith("Option<"):
        return {**shape(kind[7:-1]), "nullable": True}
    if kind.startswith("Vec<"):
        return {"type": "array", "items": shape(kind[4:-1])}
    if kind == "String":
        return {"type": "string"}
    if kind == "bool":
        return {"type": "boolean"}
    if kind in ("u32", "u64", "usize"):
        return {"type": "integer", "minimum": 0}
    return {"$ref": "#/components/schemas/" + kind.removesuffix("Dto") + "DTO"}

for name, body in re.findall(r"pub struct (Storage\w+Dto) \{(.*?)\n\}", source, re.S):
    fields = re.findall(r"pub (\w+): (.*?),\n", body)
    schemas[name.removesuffix("Dto") + "DTO"] = {
        "type": "object", "properties": {camel(n): shape(t) for n, t in fields},
        "required": [camel(n) for n, _ in fields],
    }
schemas["CapabilitiesDTO"]["properties"]["storageInventory"] = {
    "$ref": "#/components/schemas/StorageInventoryCapabilityDTO"
}
for suffix, method, name, result in [
    ("inventory", "get", "Read cached host inventory; never starts a scan", "StorageInventoryDTO"),
    ("scans", "post", "Start a bounded explicit storage scan", None),
    ("scans/{id}", "get", "Read host storage scan progress", "StorageScanDTO"),
]:
    operation = {"summary": name, "x-permission-category": "admin", "responses": {}}
    if "{id}" in suffix:
        operation["parameters"] = [{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}]
    if suffix == "inventory":
        operation["parameters"] = [{"name": "cursor", "in": "query", "schema": {"type": "string"}}]
    operation["responses"]["200" if method == "get" else "202"] = {
        "description": name,
        "content": {"application/json": {"schema": {"$ref": "#/components/schemas/" + result} if result else {
            "type": "object", "required": ["hostId", "operationId"], "properties": {
                "hostId": {"type": "string"}, "operationId": {"type": "string"}}}}},
    }
    for status in ("400", "401", "403", "404", "409", "500", "503"):
        operation["responses"][status] = {"description": "Storage operation refused or unavailable", "content": {
            "application/json": {"schema": {"$ref": "#/components/schemas/ErrorDTO"}}}}
    doc["paths"].setdefault("/v1/host/storage/" + suffix, {})[method] = operation
for suffix, method, result, body in [
    ("candidates","get","StorageCandidatesDTO",None),
    ("reviews","post","StorageReviewDTO","StorageReviewRequestDTO"),
    ("cleanups","post",None,"StorageCleanupRequestDTO"),
    ("cleanups/{id}","get","StorageReviewedResultDTO",None),
]:
    operation={"summary":"Reviewed host storage " + suffix,"x-permission-category":"admin","responses":{}}
    if "{id}" in suffix:
        operation["parameters"]=[{"name":"id","in":"path","required":True,"schema":{"type":"string"}}]
    if suffix=="candidates":
        operation["parameters"]=[{"name":n,"in":"query","required":True,"schema":{"type":"string"}} for n in ("category","action")]
    if body: operation["requestBody"]={"required":True,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/"+body}}}}
    operation["responses"]["202" if suffix=="cleanups" else "200"]={"description":"Storage review or result","content":{"application/json":{"schema":{"$ref":"#/components/schemas/"+result} if result else {"type":"object","properties":{"hostId":{"type":"string"},"operationId":{"type":"string"}},"required":["hostId","operationId"]}}}}
    for status in ("400","401","403","404","409","429","500","503"):
        operation["responses"][status]={"description":"Storage action refused","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ErrorDTO"}}}}
    doc["paths"].setdefault("/v1/host/storage/"+suffix,{})[method]=operation

def replace_member(text, key, value, container):
    match = re.search(r'(?m)^( *)("' + re.escape(key) + r'"\s*:\s*)', text)
    if match:
        start = match.end()
        _, length = json.JSONDecoder().raw_decode(text[start:])
        indent = match.group(1)
        rendered = json.dumps(value, indent=2, ensure_ascii=False).replace("\n", "\n" + indent)
        return text[:start] + rendered + text[start + length:]
    parent = re.search(r'(?m)^( *)"' + re.escape(container) + r'"\s*:\s*\{', text)
    indent = parent.group(1) + "  "
    rendered = json.dumps(value, indent=2, ensure_ascii=False).replace("\n", "\n" + indent)
    return text[:parent.end()] + "\n" + indent + json.dumps(key) + ": " + rendered + "," + text[parent.end():]

for name, value in schemas.items():
    if name.startswith("Storage") or name == "CapabilitiesDTO":
        original = replace_member(original, name, value, "schemas")
for name, value in doc["paths"].items():
    if name.startswith("/v1/host/storage/"):
        original = replace_member(original, name, value, "paths")
path.write_text(original)
