# Audio Resampling Algorithms

`rust-audio` provides several resampling algorithms so applications can trade
conversion quality, CPU cost, and filtering behavior. This guide describes the
method behind each converter rather than the API used to call it.

Resampling changes the number of samples in each channel. For every output
sample, the converter maps the output index back into the input timeline:

```text
input_position = output_index / ratio
```

When `ratio` is greater than 1, the output has more samples than the input
(upsampling). When `ratio` is less than 1, the output has fewer samples
(downsampling). The converter choice controls how the value at each mapped
input position is estimated.

## Choosing a Converter

| Converter | Method | Filtering | Cost | Best fit |
| --- | --- | --- | --- | --- |
| `ZeroOrderHold` | Reuses the previous input sample | Not bandlimited | Lowest | Rough previews, stepped/hold effects, very cheap conversion |
| `Linear` | Interpolates on a straight line between adjacent samples | Not bandlimited | Very low | Preview-quality conversion where speed matters more than fidelity |
| `SincFastest` | Bandlimited sinc interpolation with the smallest sinc table | Bandlimited | Medium | Realtime-friendly conversion with much better quality than linear |
| `SincMedium` | Bandlimited sinc interpolation with a larger sinc table | Bandlimited | Higher | Default balanced choice for general-purpose audio |
| `SincBest` | Bandlimited sinc interpolation with the largest sinc table | Bandlimited | Highest | Offline or quality-first conversion |

For downsampling real audio, prefer one of the sinc converters. `ZeroOrderHold`
and `Linear` do not apply an anti-aliasing filter, so high-frequency content can
fold back into the audible range when the output sample rate is lower.

## Zero-Order Hold

Zero-order hold is the simplest converter. It finds the mapped input position,
takes the integer part, and emits that input sample:

```text
output[n] = input[floor(n / ratio)]
```

During upsampling, this repeats samples and produces a staircase-shaped signal.
For example, doubling `[0.0, 1.0, 0.0]` produces
`[0.0, 0.0, 1.0, 1.0, 0.0, 0.0]`.

This method is extremely cheap, but it is not a high-fidelity audio resampler.
It creates strong imaging artifacts when upsampling and does not suppress
aliasing when downsampling. Use it when the stepped sound is acceptable or
intentional, or when a very rough preview is enough.

## Linear Interpolation

Linear interpolation also maps each output sample back into the input timeline,
but it uses the fractional position between the two nearest input samples:

```text
position = n / ratio
left = floor(position)
right = left + 1
fraction = position - left
output[n] = input[left] + fraction * (input[right] - input[left])
```

This replaces the staircase from zero-order hold with straight-line ramps. It is
still very fast, and it is usually smoother than zero-order hold for simple
preview work.

Linear interpolation is also not bandlimited. It reduces some obvious roughness
compared with zero-order hold, but it does not provide the low-pass filtering
needed for transparent sample-rate conversion. It can dull or distort high
frequencies, and it can alias during downsampling.

## Sinc Interpolation

The sinc converters use bandlimited interpolation. Conceptually, they reconstruct
a continuous signal from the input samples using a low-pass interpolation kernel,
then sample that reconstructed signal at the output positions.

An ideal sinc filter has infinite length, which is not practical. The
implementation uses precomputed finite sinc coefficient tables derived from
libsamplerate. For each output position, it:

1. maps the output sample to a fractional input position;
2. centers a sinc kernel around that position;
3. multiplies nearby input samples by table-derived sinc coefficients;
4. sums those weighted samples to produce the output sample.

When downsampling, the sinc path scales the filter spacing and amplitude by the
conversion ratio. This narrows the effective passband before decimation, which is
the anti-aliasing behavior missing from zero-order hold and linear
interpolation.

The three sinc converters use the same method but different coefficient tables:

| Converter | Coefficient table | Practical effect |
| --- | --- | --- |
| `SincFastest` | Smallest table | Fastest sinc option, lowest bandwidth of the sinc group |
| `SincMedium` | Larger table | Better bandwidth/quality balance, default converter |
| `SincBest` | Largest table | Best sinc quality, highest CPU and memory cost |

The sinc converters are the right choice for music, voice, archival processing,
format conversion, and any downsampling step where aliasing would be a problem.
Choose `SincFastest` when realtime cost matters, `SincMedium` for general use,
and `SincBest` when conversion quality matters more than throughput.

## Practical Guidance

Use `ZeroOrderHold` only when you can tolerate obvious artifacts or want a held,
stepped character. Use `Linear` for inexpensive previews or simple tools where
quality is secondary. Use a sinc converter for production audio, especially
when reducing sample rate.

The default converter is `SincMedium` because it is the balanced option: it is
bandlimited, avoids the major downsampling failure mode of the cheaper
converters, and costs less than `SincBest`.
