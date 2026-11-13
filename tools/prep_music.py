"""Prepara uma faixa de música para tocar em laço: corta o silêncio do fim e faz entrada e saída suaves, para a emenda
não estalar. Uso: python3 tools/prep_music.py entrada.wav saida.wav   (WAV de 16 bits)"""
import sys, wave, array
w = wave.open(sys.argv[1]); ch, sr, n = w.getnchannels(), w.getframerate(), w.getnframes()
a = array.array('h'); a.frombytes(w.readframes(n)); w.close()
peak = max(abs(v) for v in a[::50]) or 1
end = n
while end > sr and max(abs(a[(end - 1) * ch + c]) for c in range(ch)) < peak * 0.01: end -= 1
n = end; a = a[:n * ch]
fi, fo = int(1.2 * sr), int(3.0 * sr)
for i in range(fi):
    for c in range(ch): a[i * ch + c] = int(a[i * ch + c] * i / fi)
for i in range(fo):
    k = n - 1 - i
    for c in range(ch): a[k * ch + c] = int(a[k * ch + c] * i / fo)
o = wave.open(sys.argv[2], 'wb'); o.setnchannels(ch); o.setsampwidth(2); o.setframerate(sr); o.writeframes(a.tobytes()); o.close()
print('canais', ch, 'segundos', round(n / sr, 1), 'pico', peak)
