#!/usr/bin/env python3
"""Regrava falas do instrutor (ElevenLabs, voz Brian) a partir de assets/voice/script.tsv.

Uso: python3 tools/gen_voice.py id1 id2 ...   (sem argumentos: só lista o que falta)
A chave vem do Chaveiro do macOS (serviço "elevenlabs-angles"); nada dela fica no repositório.
Saída: assets/voice/<id>.mp3 em mono, 22,05 kHz, 64 kbps, como as demais.
"""
import json, os, subprocess, sys, tempfile, urllib.request
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VOICE = "nPczCjzI2devNBz1zQrb"
lines = dict(l.rstrip("\n").split("\t", 1) for l in open(os.path.join(ROOT, "assets/voice/script.tsv")) if "\t" in l)
ids = sys.argv[1:]
if not ids:
    print("faltam:", [k for k in lines if not os.path.exists(os.path.join(ROOT, "assets/voice", k + ".mp3"))]); sys.exit(0)
key = subprocess.check_output(["security", "find-generic-password", "-s", "elevenlabs-angles", "-w"], text=True).strip()
for i in ids:
    body = json.dumps({"text": lines[i], "model_id": "eleven_multilingual_v2", "voice_settings": {"stability": 0.55, "similarity_boost": 0.75, "style": 0.2}}).encode()
    req = urllib.request.Request(f"https://api.elevenlabs.io/v1/text-to-speech/{VOICE}?output_format=mp3_44100_128", data=body, headers={"xi-api-key": key, "Content-Type": "application/json"})
    data = urllib.request.urlopen(req, timeout=90).read()
    with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as f: f.write(data)
    out = os.path.join(ROOT, "assets/voice", i + ".mp3")
    subprocess.check_call(["/opt/homebrew/bin/ffmpeg", "-v", "error", "-y", "-i", f.name, "-ac", "1", "-ar", "22050", "-b:a", "64k", out])
    os.unlink(f.name)
    print(i, os.path.getsize(out))
