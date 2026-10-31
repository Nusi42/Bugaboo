// The types of the parts of `gifenc` that the recordings of the README use (the package has none).
declare module "gifenc" {
  /** Colors as red, green and blue numbers. */
  export type Palette = readonly (readonly number[])[];

  export interface FrameOptions {
    readonly palette?: Palette;
    readonly delay?: number;
    readonly transparent?: boolean;
    readonly transparentIndex?: number;
    readonly dispose?: number;
    readonly repeat?: number;
  }

  export interface Encoder {
    writeFrame(index: Buffer, width: number, height: number, options?: FrameOptions): void;
    finish(): void;
    bytes(): Uint8Array;
  }

  /** What the package exports (it is written for CommonJS, so it is taken as one object). */
  export interface Gifenc {
    GIFEncoder: () => Encoder;
    quantize: (
      rgba: Buffer,
      maxColors: number,
      options?: { readonly format?: "rgb565" | "rgb444" | "rgba4444" },
    ) => Palette;
    applyPalette: (
      rgba: Buffer,
      palette: Palette,
      format?: "rgb565" | "rgb444" | "rgba4444",
    ) => Uint8Array;
  }
}
