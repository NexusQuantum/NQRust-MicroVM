"use client";

import * as React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuSub,
  SidebarMenuSubItem,
  SidebarMenuSubButton,
} from "@/components/ui/sidebar"; // <- sesuaikan path file sidebar-mu
import { Layers, ChevronDown, ChevronRight, ListTree, FlaskConical } from "lucide-react";
import { cn } from "@/lib/utils";

export function FunctionGroup() {
  const pathname = usePathname();

  // buka otomatis saat berada di /function/*
  const defaultOpen = React.useMemo(() => pathname.startsWith("/function"), [pathname]);
  const [open, setOpen] = React.useState(defaultOpen);

  React.useEffect(() => {
    // update state saat route berubah
    setOpen(pathname.startsWith("/function"));
  }, [pathname]);

  const isActiveRoot = pathname === "/function" || pathname.startsWith("/function/");
  const isActiveList = pathname.startsWith("/function/list");
  const isActivePlay = pathname.startsWith("/function/lambda");

  return (
    <SidebarGroup>
      <SidebarGroupContent>
        <SidebarMenu>
          {/* === Root "Function" button (toggle + icon) === */}
          <SidebarMenuItem>
            <SidebarMenuButton
              onClick={() => setOpen((v) => !v)}
              isActive={isActiveRoot}
              className="justify-between -ml-2 -mt-3"
              aria-expanded={open}
            >
              <span className="flex items-center gap-2">
                <Layers className="h-4 w-4" />
                <span>Function</span>
              </span>
              {open ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
            </SidebarMenuButton>
          </SidebarMenuItem>

          {/* === Sub menu === */}
          {open && (
            <SidebarMenuSub>
              <SidebarMenuSubItem>
                <SidebarMenuSubButton
                  asChild
                  isActive={isActiveList}
                  size="md"
                  className={cn(isActiveList && "font-medium")}
                >
                  <Link href="/function/list" className="flex items-center gap-2">
                    <ListTree className="h-4 w-4" />
                    <span>List Function</span>
                  </Link>
                </SidebarMenuSubButton>
              </SidebarMenuSubItem>

              <SidebarMenuSubItem>
                <SidebarMenuSubButton
                  asChild
                  isActive={isActivePlay}
                  size="md"
                  className={cn(isActivePlay && "font-medium")}
                >
                  <Link href="/function/lambda" className="flex items-center gap-2">
                    <FlaskConical className="h-4 w-4" />
                    <span>Lambda Playground</span>
                  </Link>
                </SidebarMenuSubButton>
              </SidebarMenuSubItem>
            </SidebarMenuSub>
          )}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}
