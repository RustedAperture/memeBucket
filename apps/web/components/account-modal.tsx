"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useUser } from "@/components/user-provider";
import { useState } from "react";
import { toast } from "sonner";
import { User } from "lucide-react";
import { apiPost, apiPatch } from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

export function AccountModal() {
  const { user, refreshUser } = useUser();
  const [newUsername, setNewUsername] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [open, setOpen] = useState(false);

  async function handleLogout() {
    try {
      await apiPost("/api/auth/logout", {});
      window.location.href = "/";
    } catch (err) {
      toast.error("Failed to log out");
    }
  }

  async function handleUpdateUsername(e: React.FormEvent) {
    e.preventDefault();
    if (!newUsername.trim()) return;
    setSubmitting(true);
    
    try {
      await apiPatch("/api/account/username", { username: newUsername.trim() });
      
      await refreshUser();
      setNewUsername("");
      toast.success("Username updated successfully!");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "An error occurred");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger 
        render={
          <button className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors cursor-pointer">
            <User className="w-4 h-4" />
            <span className="hidden sm:inline">Account</span>
          </button>
        }
      />
      
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Account Settings</DialogTitle>
        </DialogHeader>
        
        <div className="space-y-6 pt-4">
          <div className="space-y-4">
            <h3 className="text-sm font-medium text-muted-foreground">Profile</h3>
            <div className="space-y-3">
              <div>
                <p className="text-sm font-medium">Current Username</p>
                <p className="text-sm text-muted-foreground">{user?.username || "Not set"}</p>
              </div>
              
              <form onSubmit={handleUpdateUsername} className="flex gap-2 items-end">
                <div className="flex-1 space-y-1">
                  <Input 
                    value={newUsername} 
                    onChange={(e) => setNewUsername(e.target.value)}
                    placeholder="New username" 
                    disabled={submitting}
                  />
                </div>
                <Button type="submit" disabled={submitting || !newUsername.trim()} size="sm">Save</Button>
              </form>
            </div>
          </div>

          <div className="space-y-4">
            <h3 className="text-sm font-medium text-muted-foreground">Account Actions</h3>
            <div className="flex gap-2 flex-wrap">
              <Button variant="secondary" onClick={handleLogout} size="sm">
                Log out
              </Button>
              <Button variant="secondary" size="sm" render={<a href="/api/account/export" />}>
                Export data
              </Button>
            </div>
          </div>

          <div className="space-y-4 border-t pt-4">
            <h3 className="text-sm font-medium text-destructive">Danger Zone</h3>
            <div>
              <Button variant="destructive" size="sm">Delete account</Button>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
