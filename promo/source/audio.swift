// Sparky promo soundtrack: 120 BPM synth track, synthesized sample by sample,
// with hits, whooshes and key clicks synced to the video timeline.
//   swiftc -O audio.swift -o audio && ./audio soundtrack.wav
import Foundation

let SR = 48000.0, DUR = 36.0
let N = Int(SR * DUR)
var L = [Float](repeating: 0, count: N), R = [Float](repeating: 0, count: N)
var rngState: UInt64 = 0x9E3779B97F4A7C15
func noise() -> Double { rngState = rngState &* 6364136223846793005 &+ 1442695040888963407; return Double(Int64(bitPattern: rngState >> 1) % 2_000_000) / 1_000_000 - 1 }
func lp(_ fc: Double) -> Double { 1 - exp(-2 * .pi * fc / SR) }

/// Adds `dur` seconds of `gen(t)` starting at `at`, panned -1…1.
func add(_ at: Double, _ dur: Double, gain: Double = 1, pan: Double = 0, _ gen: (Double) -> Double) {
    let start = Int(at * SR), count = Int(dur * SR)
    let gl = Float(gain * sqrt(0.5 * (1 - pan))), gr = Float(gain * sqrt(0.5 * (1 + pan)))
    for i in 0..<count {
        let n = start + i
        if n < 0 || n >= N { continue }
        let v = Float(gen(Double(i) / SR))
        L[n] += v * gl; R[n] += v * gr
    }
}

// MARK: instruments
func kick(_ at: Double, _ g: Double = 1) {
    var ph = 0.0
    add(at, 0.45, gain: g) { t in
        ph += 2 * .pi * (44 + 120 * exp(-t * 30)) / SR
        return sin(ph) * exp(-t * 7.5) + (t < 0.004 ? noise() * 0.4 : 0)
    }
}
func clap(_ at: Double, _ g: Double = 0.5) {
    var y = 0.0, prev = 0.0
    add(at, 0.3, gain: g, pan: 0.1) { t in
        let bursts = [0.0, 0.011, 0.022].reduce(0.0) { acc, o in acc + (t >= o ? exp(-(t - o) * (o == 0.022 ? 18 : 120)) : 0) }
        let x = noise(); let hp = x - prev; prev = x
        y += lp(2400) * (hp - y)
        return y * bursts * 2.2
    }
}
func hat(_ at: Double, _ g: Double = 0.16, open: Bool = false) {
    var prev = 0.0
    add(at, open ? 0.3 : 0.06, gain: g, pan: 0.3) { t in
        let x = noise(); let hp = x - prev; prev = x
        return hp * exp(-t * (open ? 12 : 70))
    }
}
func bass(_ at: Double, _ f: Double, _ dur: Double, _ g: Double = 0.42) {
    var ph = 0.0, y = 0.0
    add(at, dur, gain: g) { t in
        ph += 2 * .pi * f / SR
        let saw = (1...6).reduce(0.0) { $0 + sin(ph * Double($1)) / Double($1) }
        y += lp(380 + 900 * exp(-t * 14)) * (saw - y)
        let env = min(1, t / 0.004) * exp(-t * 3.5)
        return tanh(y * 1.8) * env
    }
}
func pluck(_ at: Double, _ f: Double, _ g: Double = 0.12, pan: Double = 0) {
    var ph = 0.0, y = 0.0
    add(at, 0.6, gain: g, pan: pan) { t in
        ph += 2 * .pi * f / SR
        let tri = asin(sin(ph)) * 2 / .pi + 0.3 * sin(ph * 2)
        y += lp(2600 * exp(-t * 6) + 300) * (tri - y)
        return y * min(1, t / 0.002) * exp(-t * 7)
    }
}
func pad(_ at: Double, _ notes: [Double], _ dur: Double, _ g: Double = 0.07, cutoff: Double = 1100) {
    for (k, f) in notes.enumerated() {
        for (d, pan) in [(-0.006, -0.6), (0.0, 0), (0.006, 0.6)] {
            var ph = Double(k), y = 0.0
            add(at, dur + 0.6, gain: g, pan: pan) { t in
                ph += 2 * .pi * f * (1 + d) / SR
                let saw = (1...5).reduce(0.0) { $0 + sin(ph * Double($1)) / Double($1) }
                y += lp(cutoff) * (saw - y)
                let env = min(1, t / 0.35) * (t > dur ? exp(-(t - dur) * 6) : 1)
                return y * env
            }
        }
    }
}
func whoosh(_ center: Double, _ len: Double = 0.55, _ g: Double = 0.32) {
    var y1 = 0.0, y2 = 0.0
    add(center - len / 2, len, gain: g) { t in
        let x = t / len
        let fc = 300 + 5200 * sin(.pi * x)
        y1 += lp(fc) * (noise() - y1); y2 += lp(fc * 0.5) * (y1 - y2)
        return (y1 - y2) * pow(sin(.pi * x), 2) * 2.5
    }
}
func riser(_ at: Double, _ len: Double, _ g: Double = 0.22) {
    var y = 0.0, ph = 0.0
    add(at, len, gain: g) { t in
        let x = t / len
        y += lp(400 + 7000 * x * x) * (noise() - y)
        ph += 2 * .pi * (180 + 1500 * x * x) / SR
        return (y * 0.9 + sin(ph) * 0.25) * pow(x, 2.2)
    }
}
func impact(_ at: Double, _ g: Double = 1) {
    var ph = 0.0, y = 0.0
    add(at, 2.2, gain: g) { t in
        ph += 2 * .pi * (34 + 70 * exp(-t * 9)) / SR
        y += lp(900 * exp(-t * 2) + 120) * (noise() - y)
        return sin(ph) * exp(-t * 2.4) * 0.9 + y * exp(-t * 3.5) * 0.9 + noise() * exp(-t * 40) * 0.3
    }
}
func click(_ at: Double, _ g: Double = 0.2, pan: Double = 0) {
    var ph = 0.0
    add(at, 0.03, gain: g, pan: pan) { t in
        ph += 2 * .pi * 3200 / SR
        return (noise() * exp(-t * 900) + sin(ph) * exp(-t * 220) * 0.4)
    }
}
func thud(_ at: Double, _ g: Double = 0.45) {
    var ph = 0.0
    add(at, 0.25, gain: g) { t in
        ph += 2 * .pi * (70 + 60 * exp(-t * 40)) / SR
        return sin(ph) * exp(-t * 16) + noise() * exp(-t * 300) * 0.3
    }
}
func bell(_ at: Double, _ f: Double, _ g: Double = 0.14, pan: Double = 0) {
    add(at, 1.4, gain: g, pan: pan) { t in
        sin(2 * .pi * f * t) * exp(-t * 3) + 0.5 * sin(2 * .pi * f * 2.01 * t) * exp(-t * 5) + 0.25 * sin(2 * .pi * f * 3.02 * t) * exp(-t * 8)
    }
}

// MARK: arrangement (beat = 0.5 s, bar = 2 s; drop at 4 s)
let A1 = 55.0, F1 = 43.65, C2 = 65.41, G1 = 49.0
let chords: [(Double, [Double])] = [(A1, [220, 261.63, 329.63]), (F1, [174.61, 220, 261.63]),
                                    (C2, [196, 261.63, 329.63]), (G1, [196, 246.94, 293.66])]

// Intro: Am pad swell, slams, typewriter, riser.
pad(0, chords[0].1, 4.0, 0.06, cutoff: 700)
for at in [0.15, 0.3, 0.48, 0.95, 1.1, 1.25] { thud(at, 0.35) }
for at in [2.2, 2.32, 2.44, 2.62] { thud(at, 0.5) }
whoosh(1.95, 0.6, 0.28)
for i in 0..<15 { click(3.0 + Double(i) * 0.045, 0.16, pan: Double(i % 3) * 0.2 - 0.2) }
riser(2.4, 1.6, 0.26)
whoosh(3.8, 0.5, 0.3)
impact(4.0, 1.0)

// Main groove 4–31 s, with a breath at the ⌘K keycaps (10–10.5).
for beat in 0..<54 {
    let t = 4.0 + Double(beat) * 0.5
    let inBreak = t >= 10.0 && t < 10.5
    if !inBreak { kick(t, 0.95) }
    if beat % 2 == 1 && !inBreak { clap(t, 0.42) }
    hat(t + 0.25, 0.14)
    if beat % 8 == 7 { hat(t + 0.25, 0.1, open: true) }
}
for bar in 0..<14 {
    let t0 = 4.0 + Double(bar) * 2
    let (root, notes) = chords[bar % 4]
    pad(t0, notes, 2.0, 0.045)
    for e in 0..<8 where !(t0 + Double(e) * 0.25 >= 10.0 && t0 + Double(e) * 0.25 < 10.5) {
        bass(t0 + Double(e) * 0.25 + 0.02, e % 4 == 2 ? root * 2 : root, 0.22)
    }
    if t0 >= 6.0 {   // arpeggio from the app reveal
        let arp = [notes[0] * 2, notes[1] * 2, notes[2] * 2, notes[1] * 2]
        for s in 0..<8 { pluck(t0 + Double(s) * 0.25, arp[s % 4], 0.08, pan: s % 2 == 0 ? -0.4 : 0.4) }
    }
}
// Sidechain-style duck of everything except kicks happens below via envelope.
// Transitions.
for at in [6.45, 9.95, 10.95, 13.9, 17.95, 22.85, 26.95] { whoosh(at, 0.5, 0.3) }
// ⌘K keycaps slam and press.
thud(10.0, 0.7); thud(10.2, 0.7); impact(10.5, 0.75)
// Key clicks synced to the recordings.
for f in [24, 46, 68, 96, 130, 175] { click(6.5 + Double(f) / 60, 0.22) }
for f in [62, 73, 84, 135, 144, 153, 162, 185, 200] { click(11.0 + Double(f - 30) / 60, 0.22, pan: -0.2) }
for f in [20, 55, 63, 71, 79, 87, 95, 130, 160] { click(14.0 + Double(f - 10) / 60, 0.22, pan: 0.2) }
for f in [100, 160, 244] { click(18.0 + Double(f) / 60, 0.3) }
whoosh(19.8, 0.35, 0.22); whoosh(22.2, 0.35, 0.22)
// AI chips: sparkles.
for (i, f) in [880.0, 1046.5, 1318.5, 1760].enumerated() { bell(23.5 + Double(i) * 0.5 + 0.15, f, 0.13, pan: Double(i) * 0.3 - 0.45) }
// Montage build: snare roll into the final hit.
for i in 0..<16 { let t = 30.0 + Double(i) * 0.0625; clap(t, 0.12 + 0.3 * Double(i) / 16) }
riser(29.6, 1.4, 0.2)
// Outro: final hit and a ringing Am chord.
impact(31.0, 1.0)
kick(31.0, 1.0)
pad(31.0, [220, 261.63, 329.63, 440], 4.2, 0.06, cutoff: 1500)
for (i, f) in [440.0, 523.25, 659.25, 880].enumerated() { bell(31.0 + Double(i) * 0.08, f, 0.1, pan: Double(i) * 0.25 - 0.4) }
bass(31.0, A1, 3.0, 0.4)

// Gentle duck on every main-groove kick so the low end breathes.
for n in 0..<N {
    let t = Double(n) / SR
    if t >= 4.0 && t < 31.0 {
        let since = (t - 4.0).truncatingRemainder(dividingBy: 0.5)
        let duck = Float(1 - 0.28 * exp(-since * 12) * (since > 0.01 ? 1 : 0))
        L[n] *= duck; R[n] *= duck
    }
}
// Simple stereo echo for space (3/8 note), then fade and master.
let delay = Int(0.375 * SR)
for n in delay..<N { L[n] += R[n - delay] * 0.18; R[n] += L[n - delay] * 0.18 }
for n in 0..<N {
    let t = Double(n) / SR
    let fade = Float(t > 35.0 ? max(0, (36.0 - t) / 1.0) : 1)
    L[n] = Float(tanh(Double(L[n]) * 1.15)) * fade
    R[n] = Float(tanh(Double(R[n]) * 1.15)) * fade
}
let peak = max(L.map(abs).max()!, R.map(abs).max()!)
let norm = 0.89 / peak

// MARK: WAV
var data = Data()
func u32(_ v: UInt32) { var x = v.littleEndian; data.append(Data(bytes: &x, count: 4)) }
func u16(_ v: UInt16) { var x = v.littleEndian; data.append(Data(bytes: &x, count: 2)) }
data.append("RIFF".data(using: .ascii)!); u32(UInt32(36 + N * 4)); data.append("WAVEfmt ".data(using: .ascii)!)
u32(16); u16(1); u16(2); u32(UInt32(SR)); u32(UInt32(SR) * 4); u16(4); u16(16)
data.append("data".data(using: .ascii)!); u32(UInt32(N * 4))
var pcm = [Int16](repeating: 0, count: N * 2)
for n in 0..<N {
    pcm[2 * n] = Int16(max(-1, min(1, L[n] * norm)) * 32767)
    pcm[2 * n + 1] = Int16(max(-1, min(1, R[n] * norm)) * 32767)
}
pcm.withUnsafeBytes { data.append(contentsOf: $0) }
try! data.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
print("peak before normalize \(peak), wrote \(CommandLine.arguments[1])")
