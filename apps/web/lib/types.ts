export type User = {
  id: string;
  username: string | null;
  display_name: string | null;
  avatar_url: string | null;
};

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

export type Pool = {
  id: string;
  name: string;
  share_token: string | null;
  subscriber_count: number;
  is_subscribed: boolean;
  owner_username: string | null;
  whitelist_enabled: boolean;
};
