import { AppShell } from "@/components/app-shell";
import { Card, CardContent } from "@/components/ui/card";

type StatusPageProps = {
  code: string;
  title: string;
  description: string;
  children: React.ReactNode;
};

export function StatusPage({ code, title, description, children }: StatusPageProps) {
  return (
    <AppShell>
      <div className="flex min-h-full items-center justify-center py-8">
        <Card className="w-full max-w-lg text-center">
          <CardContent className="flex flex-col items-center gap-5 p-8">
            <p className="text-7xl font-bold tracking-tighter text-primary">{code}</p>
            <div className="space-y-2">
              <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
              <p className="text-muted-foreground">{description}</p>
            </div>
            <div className="flex flex-wrap justify-center gap-3">{children}</div>
          </CardContent>
        </Card>
      </div>
    </AppShell>
  );
}
