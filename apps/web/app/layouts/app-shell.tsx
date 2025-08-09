import { Outlet, Link, useLocation } from "react-router";
import { useEffect, useMemo, useState } from "react";
import { cn } from "@revlentless/ui/lib/utils";
import { Toaster } from "@revlentless/ui/components/sonner";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@revlentless/ui/components/breadcrumb";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@revlentless/ui/components/dropdown-menu";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@revlentless/ui/components/sheet";
import { Input } from "@revlentless/ui/components/input";
import { Button } from "@revlentless/ui/components/button";
import { Avatar, AvatarFallback } from "@revlentless/ui/components/avatar";
import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@revlentless/ui/components/command";
import { Separator } from "@revlentless/ui/components/separator";
import { ThemeToggle } from "@revlentless/ui-theme/components/theme-toggle";
import {
  Menu,
  Search,
  SquareDashedMousePointer,
  LayoutDashboard,
  BookOpen,
  Settings,
  LogOut,
} from "lucide-react";

const nav = [
  { href: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { href: "/sessions", label: "Sessions", icon: SquareDashedMousePointer },
  { href: "/topics", label: "Topics", icon: BookOpen },
  { href: "/settings", label: "Settings", icon: Settings },
] as const;

export default function AppShell() {
  const location = useLocation();
  const pathname = location.pathname;
  const [open, setOpen] = useState(false);
  const [cmdOpen, setCmdOpen] = useState(false);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setCmdOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const crumbs = useMemo(() => {
    const parts = pathname.split("/").filter(Boolean);
    const acc: { href: string; name: string }[] = [];
    parts.forEach((p, i) => {
      const href = "/" + parts.slice(0, i + 1).join("/");
      acc.push({ href, name: p });
    });
    return acc;
  }, [pathname]);

  return (
    <div className="min-h-[100svh]">
      <header className="sticky top-0 z-40 border-b bg-background/80 backdrop-blur">
        <div className="mx-auto flex max-w-[1400px] items-center gap-3 px-4 py-3">
          <div className="flex items-center gap-2">
            <Sheet open={open} onOpenChange={setOpen}>
              <SheetTrigger asChild className="lg:hidden">
                <Button variant="outline" size="icon">
                  <Menu className="h-4 w-4" />
                </Button>
              </SheetTrigger>
              <SheetContent side="left" className="w-72 p-0">
                <SheetHeader className="p-4">
                  <SheetTitle>Feynman</SheetTitle>
                </SheetHeader>
                <Separator />
                <nav className="p-2">
                  {nav.map((item) => {
                    const Icon = item.icon;
                    const active = pathname.startsWith(item.href);
                    return (
                      <Link
                        key={item.href}
                        to={item.href}
                        className={cn(
                          "flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-muted",
                          active && "bg-muted font-medium"
                        )}
                        onClick={() => setOpen(false)}
                      >
                        <Icon className="h-4 w-4" />
                        {item.label}
                      </Link>
                    );
                  })}
                </nav>
              </SheetContent>
            </Sheet>

            <Link to="/dashboard" className="hidden items-center gap-2 lg:flex">
              <div className="flex h-8 w-8 items-center justify-center rounded bg-emerald-600 font-bold text-white">
                F
              </div>
              <span className="text-sm font-semibold">Feynman</span>
            </Link>
          </div>

          <div className="flex flex-1 items-center gap-2">
            <div className="relative hidden flex-1 items-center lg:flex">
              <Search className="pointer-events-none absolute left-3 h-4 w-4 text-muted-foreground" />
              <Input
                className="pl-9"
                placeholder="Search sessions, topics, settings..."
                onFocus={() => setCmdOpen(true)}
              />
            </div>
            <Button
              variant="outline"
              size="sm"
              className="lg:hidden"
              onClick={() => setCmdOpen(true)}
            >
              <Search className="mr-2 h-4 w-4" />
              Search
            </Button>
          </div>

          <div className="ml-auto flex items-center gap-2">
            <ThemeToggle />
            <UserMenu />
          </div>
        </div>
      </header>

      <div className="mx-auto grid max-w-[1400px] grid-cols-1 gap-6 px-4 py-6 lg:grid-cols-[240px_minmax(0,1fr)]">
        <aside className="sticky top-[64px] hidden self-start lg:block">
          <nav className="rounded-lg border">
            {nav.map((item) => {
              const Icon = item.icon;
              const active = pathname.startsWith(item.href);
              return (
                <Link
                  key={item.href}
                  to={item.href}
                  className={cn(
                    "flex items-center gap-2 border-b px-4 py-3 text-sm hover:bg-muted first:rounded-t-lg last:rounded-b-lg last:border-b-0",
                    active && "bg-muted font-medium"
                  )}
                >
                  <Icon className="h-4 w-4" />
                  {item.label}
                </Link>
              );
            })}
          </nav>
        </aside>
        <main>
          <div className="mb-4 hidden lg:block">
            <Breadcrumb>
              <BreadcrumbList>
                {crumbs.map((c, idx) => (
                  <BreadcrumbItem key={c.href}>
                    {idx < crumbs.length - 1 ? (
                      <>
                        <BreadcrumbLink asChild>
                          <Link to={c.href}>{pretty(c.name)}</Link>
                        </BreadcrumbLink>
                        <BreadcrumbSeparator />
                      </>
                    ) : (
                      <BreadcrumbPage>{pretty(c.name)}</BreadcrumbPage>
                    )}
                  </BreadcrumbItem>
                ))}
              </BreadcrumbList>
            </Breadcrumb>
          </div>
          <Outlet />
        </main>
      </div>

      <CommandDialog open={cmdOpen} onOpenChange={setCmdOpen}>
        <Command>
          <CommandInput placeholder="Type a command or search..." />
          <CommandList>
            <CommandEmpty>No results found.</CommandEmpty>
            <CommandGroup heading="Navigation">
              {nav.map((n) => (
                <CommandItem
                  key={n.href}
                  onSelect={() => (window.location.href = n.href)}
                >
                  <n.icon className="mr-2 h-4 w-4" />
                  <span>{n.label}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </CommandDialog>

      <Toaster richColors position="top-right" />
    </div>
  );
}

function pretty(seg: string) {
  if (seg === "app") return "App";
  if (seg === "sessions") return "Sessions";
  return seg.replace(/^\w/, (s) => s.toUpperCase());
}

function UserMenu() {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" className="gap-2">
          <Avatar className="h-5 w-5">
            <AvatarFallback>AL</AvatarFallback>
          </Avatar>
          <span className="hidden sm:inline">Ada Lovelace</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>My Account</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuItem asChild>
            <Link to="/app/settings">Settings</Link>
          </DropdownMenuItem>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem className="text-red-600">
          <LogOut className="mr-2 h-4 w-4" />
          Log out (mock)
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
