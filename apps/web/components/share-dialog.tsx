"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { apiPost, apiGet, apiDelete, apiPatch } from "@/lib/api";
import { Pool } from "@/lib/types";
import { Copy, KeyRound, Globe, Lock, Share2, Check, Users, Plus, X } from "lucide-react";
import { toast } from "sonner";

export function ShareDialog({ pool, onPoolChange }: { pool: Pool; onPoolChange: (pool: Pool) => void }) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const [whitelistEnabled, setWhitelistEnabled] = useState(pool.whitelist_enabled);
  const [whitelistedUsers, setWhitelistedUsers] = useState<string[]>([]);
  const [newUsername, setNewUsername] = useState("");
  const [whitelistLoaded, setWhitelistLoaded] = useState(false);

  useEffect(() => {
    if (whitelistEnabled && pool.share_token && !whitelistLoaded) {
      void loadWhitelist();
    }
  }, [whitelistEnabled, pool.share_token, whitelistLoaded]);

  async function loadWhitelist() {
    try {
      const users = await apiGet<string[]>(`/api/pools/${pool.id}/whitelist`);
      setWhitelistedUsers(users);
      setWhitelistLoaded(true);
    } catch (err) {
      console.error(err);
    }
  }

  async function handleToggleWhitelist(checked: boolean) {
    setWhitelistEnabled(checked);
    onPoolChange({ ...pool, whitelist_enabled: checked });
    try {
      await apiPatch(`/api/pools/${pool.id}/whitelist-enabled`, { enabled: checked });
    } catch (err) {
      setWhitelistEnabled(!checked);
      onPoolChange({ ...pool, whitelist_enabled: !checked });
      toast.error("Failed to update whitelist setting");
    }
  }

  async function handleAddUser(e: React.FormEvent) {
    e.preventDefault();
    if (!newUsername.trim()) return;
    try {
      await apiPost(`/api/pools/${pool.id}/whitelist`, { username: newUsername.trim() });
      setNewUsername("");
      await loadWhitelist();
      toast.success("User added to whitelist");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "User not found or already added");
    }
  }

  async function handleRemoveUser(username: string) {
    try {
      await apiDelete(`/api/pools/${pool.id}/whitelist/${encodeURIComponent(username)}`);
      await loadWhitelist();
    } catch (err) {
      toast.error("Failed to remove user");
    }
  }

  async function handleEnable() {
    setLoading(true);
    setError(null);
    try {
      const res = await apiPost<unknown, { share_token: string }>(`/api/pools/${pool.id}/share`, {});
      onPoolChange({ ...pool, share_token: res.share_token });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to enable sharing");
    } finally {
      setLoading(false);
    }
  }

  async function handleDisable() {
    if (!confirm("Are you sure? Existing links will stop working.")) return;
    setLoading(true);
    setError(null);
    try {
      await apiPost(`/api/pools/${pool.id}/unshare`, {});
      onPoolChange({ ...pool, share_token: null });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to disable sharing");
    } finally {
      setLoading(false);
    }
  }

  async function handleCopy() {
    if (!pool.share_token) return;
    const shareUrl = `${window.location.origin}/share?token=${pool.share_token}`;
    await navigator.clipboard.writeText(shareUrl);
    toast.success("Share link copied to clipboard!");
  }

  async function handleCopyCode() {
    if (!pool.share_token) return;
    await navigator.clipboard.writeText(pool.share_token);
    toast.success("Code copied to clipboard!");
  }

  return (
    <div className="flex flex-col gap-4 mt-2">
      {error && <div className="text-sm font-medium text-destructive">{error}</div>}
      
      {!pool.share_token ? (
        <div className="flex flex-col items-center text-center p-6 bg-muted/30 rounded-xl border border-dashed">
          <div className="w-12 h-12 rounded-full bg-muted flex items-center justify-center mb-4">
            <Lock className="w-6 h-6 text-muted-foreground" />
          </div>
          <h3 className="font-semibold text-lg mb-2">Sharing is disabled</h3>
          <p className="text-sm text-muted-foreground mb-6 max-w-sm">
            Enable sharing to generate a short code that others can use to subscribe to this pool.
          </p>
          <Button onClick={handleEnable} disabled={loading} className="w-full">
            <Globe className="w-4 h-4 mr-2" />
            Enable Sharing
          </Button>
        </div>
      ) : (
        <div className="flex flex-col gap-6">
          <div className="flex items-center justify-between">
            <h3 className="font-semibold">Share Settings</h3>
            <div className="flex items-center text-sm text-muted-foreground bg-muted/50 px-2 py-1 rounded-full">
              <Users className="w-4 h-4 mr-1.5" />
              {pool.subscriber_count} subscriber{pool.subscriber_count !== 1 ? 's' : ''}
            </div>
          </div>
          
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">Short Code</label>
            <div className="flex items-center gap-2">
              <div className="flex-1 relative">
                <KeyRound className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
                <Input 
                  readOnly 
                  value={pool.share_token} 
                  className="pl-9 font-mono text-center tracking-wider text-lg font-bold" 
                />
              </div>
              <Button onClick={handleCopyCode} variant="outline" size="icon">
                <Copy className="w-4 h-4" />
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Users can enter this code in the "Join Existing" tab.
            </p>
          </div>

          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">Share Link</label>
            <div className="flex gap-2">
              <Button onClick={handleCopy} className="flex-1" variant="secondary">
                <Share2 className="w-4 h-4 mr-2" />
                Copy Link
              </Button>
            </div>
          </div>

          <div className="pt-4 border-t border-border/50 flex flex-col gap-4">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="whitelist-toggle">Require Whitelist</Label>
                <p className="text-xs text-muted-foreground">Only specific users can join.</p>
              </div>
              <Switch id="whitelist-toggle" checked={whitelistEnabled} onCheckedChange={handleToggleWhitelist} />
            </div>
            
            {whitelistEnabled && (
              <div className="space-y-3 bg-muted/30 p-3 rounded-lg border">
                <form onSubmit={handleAddUser} className="flex gap-2">
                  <Input 
                    placeholder="Enter username..." 
                    value={newUsername} 
                    onChange={e => setNewUsername(e.target.value)} 
                    className="h-8 text-sm"
                  />
                  <Button type="submit" size="sm" className="h-8 px-2" disabled={!newUsername.trim()}>
                    <Plus className="h-4 w-4" />
                  </Button>
                </form>
                
                <div className="flex flex-wrap gap-2">
                  {whitelistedUsers.length === 0 && (
                    <span className="text-xs text-muted-foreground italic">No users added yet.</span>
                  )}
                  {whitelistedUsers.map(user => (
                    <div key={user} className="flex items-center gap-1 bg-secondary text-secondary-foreground px-2 py-1 rounded-md text-xs">
                      {user}
                      <button onClick={() => handleRemoveUser(user)} className="text-muted-foreground hover:text-foreground">
                        <X className="h-3 w-3" />
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          <div className="pt-4 border-t border-border/50">
            <Button onClick={handleDisable} disabled={loading} variant="destructive" className="w-full">
              <Lock className="w-4 h-4 mr-2" />
              Disable Sharing
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
