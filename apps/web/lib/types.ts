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

export type ImageItem = {
  id: string;
  url: string;
  title: string | null;
  favorite: boolean;
  randomWeight: number;
  tags: string[];
  sendCount: number;
  createdAt?: string;
  notes?: string | null;
};

export type GifSearchSelection = {
  url: string;
  title: string | null;
  slug: string | null;
  tags: string[];
};

export type ImageSearchResult = {
  bucketId: string;
  bucketName: string;
  image: ImageItem;
};

export type Bucket = {
  id: string;
  name: string;
  share_token: string | null;
  subscriber_count: number;
  is_subscribed: boolean;
  owner_username: string | null;
  whitelist_enabled: boolean;
  image_count: number;
  is_read_only: boolean;
};
