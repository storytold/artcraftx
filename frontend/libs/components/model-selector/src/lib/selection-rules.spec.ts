import { GenerationProvider } from "@storyteller/api-enums";
import { ImageModel, ModelCreator } from "@storyteller/model-list";
import {
  chooseAccountForProvider,
  chooseModelForProvider,
  chooseProviderForModel,
  providerForService,
} from "./selection-rules";

const model = (id: string, providers: GenerationProvider[]) =>
  new ImageModel({
    id, tauriId: id, fullName: id, category: "image", creator: ModelCreator.ArtCraft,
    selectorName: id, selectorDescription: "", selectorBadges: [],
    maxGenerationCount: 1, defaultGenerationCount: 1, providers,
  });

const midjourney7 = model("midjourney_7", [GenerationProvider.Artcraft, GenerationProvider.Midjourney]);
const grokImagine = model("grok_imagine_image", [GenerationProvider.Grok]);
const flux = model("flux_1_dev", [GenerationProvider.Artcraft, GenerationProvider.Fal]);
const page = [midjourney7, flux, grokImagine];

const accounts = [
  { id: "c_artcraft", service: "artcraft" },
  { id: "c_grok", service: "grok_cookies" },
  { id: "c_mj", service: "midjourney_cookies" },
  { id: "c_runway", service: "runway_cookies" },
];

describe("providerForService", () => {
  it("maps credential services to providers", () => {
    expect(providerForService("grok_cookies")).toBe(GenerationProvider.Grok);
    expect(providerForService("midjourney_cookies")).toBe(GenerationProvider.Midjourney);
    expect(providerForService("artcraft_api")).toBe(GenerationProvider.Artcraft);
    expect(providerForService("runway_cookies")).toBeUndefined();
  });
});

describe("chooseProviderForModel", () => {
  it("keeps the current provider when the model is offered there", () => {
    expect(chooseProviderForModel(midjourney7, GenerationProvider.Midjourney)).toBe(GenerationProvider.Midjourney);
  });
  it("falls back to the model's default provider otherwise", () => {
    expect(chooseProviderForModel(midjourney7, GenerationProvider.Grok)).toBe(GenerationProvider.Artcraft);
    expect(chooseProviderForModel(grokImagine, undefined)).toBe(GenerationProvider.Grok);
  });
});

describe("chooseModelForProvider", () => {
  it("picks the first page model the provider offers", () => {
    expect(chooseModelForProvider(GenerationProvider.Grok, page)).toBe(grokImagine);
    expect(chooseModelForProvider(GenerationProvider.Fal, page)).toBe(flux);
  });
  it("is undefined when the provider offers nothing on the page", () => {
    expect(chooseModelForProvider(GenerationProvider.WorldLabs, page)).toBeUndefined();
  });
});

describe("chooseAccountForProvider", () => {
  it("keeps the current account when it matches the provider", () => {
    expect(chooseAccountForProvider(GenerationProvider.Grok, accounts, "c_grok")?.id).toBe("c_grok");
  });
  it("switches to the first account for the provider otherwise", () => {
    expect(chooseAccountForProvider(GenerationProvider.Midjourney, accounts, "c_grok")?.id).toBe("c_mj");
    expect(chooseAccountForProvider(GenerationProvider.Fal, accounts, "c_grok")).toBeUndefined();
  });
});
