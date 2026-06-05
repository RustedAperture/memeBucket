export type CategorySummary = {
  id: string;
  name: string;
  linkCount: number;
  timesUsed: number;
  lastUsedAt: string | null;
};

export type MediaLink = {
  id: string;
  url: string;
  previewStatus: "unchecked" | "ok" | "warning" | "failed";
};
