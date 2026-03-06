"use client"

import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { Slot } from "radix-ui"

import { useIsMobile } from "@/hooks/use-mobile"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Separator } from "@/components/ui/separator"
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet"
import { Skeleton } from "@/components/ui/skeleton"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { PanelLeftIcon } from "lucide-react"

const SIDEBAR_COOKIE_NAME = "sidebar_state"
const SIDEBAR_COOKIE_MAX_AGE = 60 * 60 * 24 * 7
const SIDEBAR_WIDTH_MOBILE = "18rem"
const SIDEBAR_WIDTH_ICON = "3rem"
const SIDEBAR_KEYBOARD_SHORTCUT = "b"

// Resizable sidebar constants
const SIDEBAR_WIDTH_MIN_LEFT = 160
const SIDEBAR_WIDTH_MIN_RIGHT = 360
const SIDEBAR_WIDTH_MAX = 480
const SIDEBAR_WIDTH_MAX_RIGHT_RATIO = 0.4
const SIDEBAR_WIDTH_DEFAULT_LEFT = 208
const SIDEBAR_WIDTH_DEFAULT_RIGHT = 400
const SIDEBAR_STORAGE_KEY_LEFT = "sidebar-width-left"
const SIDEBAR_STORAGE_KEY_RIGHT = "sidebar-width-right"

const getSidebarMaxWidth = (side: "left" | "right") => {
  if (side === "left") {
    return SIDEBAR_WIDTH_MAX
  }

  const viewportWidth =
    typeof window !== "undefined" ? window.innerWidth : SIDEBAR_WIDTH_MAX
  return Math.max(
    SIDEBAR_WIDTH_MIN_RIGHT,
    Math.floor(viewportWidth * SIDEBAR_WIDTH_MAX_RIGHT_RATIO)
  )
}

const getSidebarMinWidth = (side: "left" | "right") =>
  side === "left" ? SIDEBAR_WIDTH_MIN_LEFT : SIDEBAR_WIDTH_MIN_RIGHT

type SidebarSetOpen = React.Dispatch<React.SetStateAction<boolean>>

type SidebarLeftContextProps = {
  leftState: "expanded" | "collapsed"
  leftOpen: boolean
  setLeftOpen: SidebarSetOpen
  toggleLeft: () => void
  leftWidth: number
  setLeftWidth: (width: number) => void
  leftResizing: boolean
  setLeftResizing: (resizing: boolean) => void
}

type SidebarRightContextProps = {
  rightState: "expanded" | "collapsed"
  rightOpen: boolean
  setRightOpen: SidebarSetOpen
  toggleRight: () => void
  rightWidth: number
  setRightWidth: (width: number) => void
  rightResizing: boolean
  setRightResizing: (resizing: boolean) => void
}

type SidebarMobileContextProps = {
  isMobile: boolean
  openMobile: boolean
  setOpenMobile: SidebarSetOpen
}

const SidebarLeftContext = React.createContext<SidebarLeftContextProps | null>(null)
const SidebarRightContext = React.createContext<SidebarRightContextProps | null>(null)
const SidebarMobileContext = React.createContext<SidebarMobileContextProps | null>(null)

function useSidebarLeft() {
  const context = React.useContext(SidebarLeftContext)
  if (!context) {
    throw new Error("useSidebarLeft must be used within a SidebarProvider.")
  }

  return context
}

function useSidebarRight() {
  const context = React.useContext(SidebarRightContext)
  if (!context) {
    throw new Error("useSidebarRight must be used within a SidebarProvider.")
  }

  return context
}

function useSidebarMobile() {
  const context = React.useContext(SidebarMobileContext)
  if (!context) {
    throw new Error("useSidebarMobile must be used within a SidebarProvider.")
  }

  return context
}

function useSidebar() {
  const left = useSidebarLeft()
  const right = useSidebarRight()
  const mobile = useSidebarMobile()

  return {
    ...left,
    ...right,
    ...mobile,
  }
}

function SidebarProvider({
  defaultLeftOpen = true,
  defaultRightOpen = false,
  defaultLeftWidth = SIDEBAR_WIDTH_DEFAULT_LEFT,
  defaultRightWidth = SIDEBAR_WIDTH_DEFAULT_RIGHT,
  leftOpen: leftOpenProp,
  rightOpen: rightOpenProp,
  onLeftOpenChange,
  onRightOpenChange,
  className,
  style,
  children,
  ...props
}: React.ComponentProps<"div"> & {
  defaultLeftOpen?: boolean
  defaultRightOpen?: boolean
  defaultLeftWidth?: number
  defaultRightWidth?: number
  leftOpen?: boolean
  rightOpen?: boolean
  onLeftOpenChange?: (open: boolean) => void
  onRightOpenChange?: (open: boolean) => void
}) {
  const isMobile = useIsMobile()
  const [openMobile, setOpenMobile] = React.useState(false)

  // Left sidebar state
  const [_leftOpen, _setLeftOpen] = React.useState(defaultLeftOpen)
  const leftOpen = leftOpenProp ?? _leftOpen
  const leftOpenRef = React.useRef(leftOpen)
  React.useEffect(() => {
    leftOpenRef.current = leftOpen
  }, [leftOpen])

  const setLeftOpen = React.useCallback(
    (value: React.SetStateAction<boolean>) => {
      const current = leftOpenRef.current
      const openState = typeof value === "function" ? value(current) : value
      if (onLeftOpenChange) {
        onLeftOpenChange(openState)
      } else {
        _setLeftOpen(openState)
      }

      // This sets the cookie to keep the sidebar state.
      document.cookie = `${SIDEBAR_COOKIE_NAME}=${openState}; path=/; max-age=${SIDEBAR_COOKIE_MAX_AGE}`
    },
    [onLeftOpenChange]
  )

  // Right sidebar state
  const [_rightOpen, _setRightOpen] = React.useState(defaultRightOpen)
  const rightOpen = rightOpenProp ?? _rightOpen
  const rightOpenRef = React.useRef(rightOpen)
  React.useEffect(() => {
    rightOpenRef.current = rightOpen
  }, [rightOpen])

  const setRightOpen = React.useCallback(
    (value: React.SetStateAction<boolean>) => {
      const current = rightOpenRef.current
      const openState = typeof value === "function" ? value(current) : value
      if (onRightOpenChange) {
        onRightOpenChange(openState)
      } else {
        _setRightOpen(openState)
      }
    },
    [onRightOpenChange]
  )

  // Left sidebar width state with localStorage persistence
  const [leftWidth, setLeftWidthState] = React.useState(() => {
    if (typeof window === "undefined") return defaultLeftWidth
    const stored = localStorage.getItem(SIDEBAR_STORAGE_KEY_LEFT)
    if (stored) {
      const parsed = parseInt(stored, 10)
      if (!isNaN(parsed)) {
        const maxLeft = SIDEBAR_WIDTH_MAX
        const minLeft = Math.min(SIDEBAR_WIDTH_MIN_LEFT, maxLeft)
        return Math.max(minLeft, Math.min(maxLeft, parsed))
      }
    }
    return defaultLeftWidth
  })

  // Right sidebar width state with localStorage persistence
  const [rightWidth, setRightWidthState] = React.useState(() => {
    if (typeof window === "undefined") return defaultRightWidth
    const stored = localStorage.getItem(SIDEBAR_STORAGE_KEY_RIGHT)
    if (stored) {
      const parsed = parseInt(stored, 10)
      if (!isNaN(parsed)) {
        const maxRight = getSidebarMaxWidth("right")
        const minRight = getSidebarMinWidth("right")
        return Math.max(minRight, Math.min(maxRight, parsed))
      }
    }
    return defaultRightWidth
  })

  // Constrained setWidth functions
  const setLeftWidth = React.useCallback((width: number) => {
    const maxLeft = SIDEBAR_WIDTH_MAX
    const minLeft = Math.min(SIDEBAR_WIDTH_MIN_LEFT, maxLeft)
    const constrained = Math.max(minLeft, Math.min(maxLeft, width))
    setLeftWidthState(constrained)
    localStorage.setItem(SIDEBAR_STORAGE_KEY_LEFT, String(constrained))
  }, [])

  const setRightWidth = React.useCallback((width: number) => {
    const maxRight = getSidebarMaxWidth("right")
    const minRight = getSidebarMinWidth("right")
    const constrained = Math.max(minRight, Math.min(maxRight, width))
    setRightWidthState(constrained)
    localStorage.setItem(SIDEBAR_STORAGE_KEY_RIGHT, String(constrained))
  }, [])

  React.useEffect(() => {
    const minLeft = getSidebarMinWidth("left")
    if (leftWidth < minLeft) {
      setLeftWidth(minLeft)
    }
  }, [leftWidth, setLeftWidth])

  React.useEffect(() => {
    const minRight = getSidebarMinWidth("right")
    const maxRight = getSidebarMaxWidth("right")
    if (rightWidth < minRight) {
      setRightWidth(minRight)
      return
    }
    if (rightWidth > maxRight) {
      setRightWidth(maxRight)
    }
  }, [rightWidth, setRightWidth])

  // Helper to toggle the left sidebar.
  const toggleLeft = React.useCallback(() => {
    setLeftOpen((open) => !open)
    // Close right sidebar when toggling left
    setRightOpen(false)
  }, [setLeftOpen, setRightOpen])

  // Helper to toggle the right sidebar.
  const toggleRight = React.useCallback(() => {
    setRightOpen((open) => !open)
    // Close left sidebar when toggling right
    setLeftOpen(false)
  }, [setLeftOpen, setRightOpen])

  const toggleLeftRef = React.useRef(toggleLeft)
  React.useEffect(() => {
    toggleLeftRef.current = toggleLeft
  }, [toggleLeft])

  // Adds a keyboard shortcut to toggle the left sidebar.
  React.useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (
        event.key === SIDEBAR_KEYBOARD_SHORTCUT &&
        (event.metaKey || event.ctrlKey)
      ) {
        event.preventDefault()
        toggleLeftRef.current()
      }
    }

    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [])

  // Resizing state for smooth drag experience
  const [leftResizing, setLeftResizing] = React.useState(false)
  const [rightResizing, setRightResizing] = React.useState(false)

  // We add a state so that we can do data-state="expanded" or "collapsed".
  // This makes it easier to style the sidebar with Tailwind classes.
  const leftState = leftOpen ? "expanded" : "collapsed"
  const rightState = rightOpen ? "expanded" : "collapsed"

  const leftContextValue = React.useMemo<SidebarLeftContextProps>(
    () => ({
      leftState,
      leftOpen,
      setLeftOpen,
      toggleLeft,
      leftWidth,
      setLeftWidth,
      leftResizing,
      setLeftResizing,
    }),
    [leftState, leftOpen, setLeftOpen, toggleLeft, leftWidth, setLeftWidth, leftResizing]
  )

  const rightContextValue = React.useMemo<SidebarRightContextProps>(
    () => ({
      rightState,
      rightOpen,
      setRightOpen,
      toggleRight,
      rightWidth,
      setRightWidth,
      rightResizing,
      setRightResizing,
    }),
    [rightState, rightOpen, setRightOpen, toggleRight, rightWidth, setRightWidth, rightResizing]
  )

  const mobileContextValue = React.useMemo<SidebarMobileContextProps>(
    () => ({
      isMobile,
      openMobile,
      setOpenMobile,
    }),
    [isMobile, openMobile, setOpenMobile]
  )

  return (
    <SidebarLeftContext.Provider value={leftContextValue}>
      <SidebarRightContext.Provider value={rightContextValue}>
        <SidebarMobileContext.Provider value={mobileContextValue}>
          <div
            data-slot="sidebar-wrapper"
            style={
              {
                "--sidebar-width": `${leftWidth}px`,
                "--sidebar-width-right": `${rightWidth}px`,
                "--sidebar-width-icon": SIDEBAR_WIDTH_ICON,
                ...style,
              } as React.CSSProperties
            }
            className={cn(
              "group/sidebar-wrapper has-data-[variant=inset]:bg-sidebar flex min-h-svh w-full",
              className
            )}
            {...props}
          >
            {children}
          </div>
        </SidebarMobileContext.Provider>
      </SidebarRightContext.Provider>
    </SidebarLeftContext.Provider>
  )
}

function Sidebar({
  side = "left",
  variant = "sidebar",
  collapsible = "offcanvas",
  className,
  children,
  dir,
  style,
  ...props
}: React.ComponentProps<"div"> & {
  side?: "left" | "right"
  variant?: "sidebar" | "floating" | "inset"
  collapsible?: "offcanvas" | "icon" | "none"
}) {
  const { isMobile, openMobile, setOpenMobile } = useSidebarMobile()
  const { leftState, leftWidth, leftResizing } = useSidebarLeft()
  const { rightState, rightWidth, rightResizing } = useSidebarRight()

  // Get the correct state and width based on side
  const state = side === "left" ? leftState : rightState
  const width = side === "left" ? leftWidth : rightWidth
  const isResizing = side === "left" ? leftResizing : rightResizing
  const cssVar = side === "left" ? "--sidebar-width" : "--sidebar-width-right"

  if (collapsible === "none") {
    return (
      <div
        data-slot="sidebar"
        style={
          {
            [cssVar]: `${width}px`,
            ...style,
          } as React.CSSProperties
        }
        className={cn(
          "bg-sidebar text-sidebar-foreground flex h-full w-(--sidebar-width) flex-col",
          side === "right" && "w-[var(--sidebar-width-right)]",
          className
        )}
        {...props}
      >
        {children}
      </div>
    )
  }

  if (isMobile) {
    return (
      <Sheet open={openMobile} onOpenChange={setOpenMobile} {...props}>
        <SheetContent
          dir={dir}
          data-sidebar="sidebar"
          data-slot="sidebar"
          data-mobile="true"
          className="bg-sidebar text-sidebar-foreground w-(--sidebar-width) p-0 [&>button]:hidden"
          style={
            {
              "--sidebar-width": SIDEBAR_WIDTH_MOBILE,
            } as React.CSSProperties
          }
          side={side}
        >
          <SheetHeader className="sr-only">
            <SheetTitle>Sidebar</SheetTitle>
            <SheetDescription>Displays the mobile sidebar.</SheetDescription>
          </SheetHeader>
          <div className="flex h-full w-full flex-col">{children}</div>
        </SheetContent>
      </Sheet>
    )
  }

  return (
    <div
      className="group peer text-sidebar-foreground hidden md:block"
      data-state={state}
      data-collapsible={state === "collapsed" ? collapsible : ""}
      data-variant={variant}
      data-side={side}
      data-slot="sidebar"
      data-resizing={isResizing || undefined}
      style={
        {
          [cssVar]: `${width}px`,
          ...style,
        } as React.CSSProperties
      }
    >
      {/* This is what handles the sidebar gap on desktop */}
      <div
        data-slot="sidebar-gap"
        className={cn(
          "relative bg-transparent",
          !isResizing && "transition-[width] duration-200 ease-linear",
          side === "left" ? "w-(--sidebar-width)" : "w-[var(--sidebar-width-right)]",
          "group-data-[collapsible=offcanvas]:w-0",
          "group-data-[side=right]:rotate-180",
          variant === "floating" || variant === "inset"
            ? "group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+(--spacing(4)))]"
            : "group-data-[collapsible=icon]:w-(--sidebar-width-icon)"
        )}
      />
      <div
        data-slot="sidebar-container"
        data-side={side}
        className={cn(
          "fixed inset-y-0 z-10 hidden h-svh md:flex",
          !isResizing && "transition-[left,right,width] duration-200 ease-linear",
          side === "left" ? "left-0 w-(--sidebar-width) group-data-[collapsible=offcanvas]:left-[calc(var(--sidebar-width)*-1)]" : "right-0 w-[var(--sidebar-width-right)] group-data-[collapsible=offcanvas]:right-[calc(var(--sidebar-width-right)*-1)]",
          // Adjust the padding for floating and inset variants.
          variant === "floating" || variant === "inset"
            ? "p-2 group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+(--spacing(4))+2px)]"
            : "group-data-[collapsible=icon]:w-(--sidebar-width-icon) group-data-[side=left]:border-r group-data-[side=right]:border-l",
          className
        )}
        {...props}
      >
        <div
          data-sidebar="sidebar"
          data-slot="sidebar-inner"
          className="bg-sidebar group-data-[variant=floating]:ring-sidebar-border group-data-[variant=floating]:rounded-lg group-data-[variant=floating]:shadow-sm group-data-[variant=floating]:ring-1 flex size-full flex-col"
        >
          {children}
        </div>
      </div>
    </div>
  )
}

function SidebarTrigger({
  className,
  onClick,
  side = "left",
  ...props
}: React.ComponentProps<typeof Button> & { side?: "left" | "right" }) {
  const { toggleLeft } = useSidebarLeft()
  const { toggleRight } = useSidebarRight()

  const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    onClick?.(event)
    if (side === "left") {
      toggleLeft()
    } else {
      toggleRight()
    }
  }

  return (
    <Button
      data-sidebar="trigger"
      data-slot="sidebar-trigger"
      variant="ghost"
      size="icon-sm"
      className={cn(className)}
      onClick={handleClick}
      {...props}
    >
      <PanelLeftIcon />
      <span className="sr-only">Toggle Sidebar</span>
    </Button>
  )
}

function SidebarRail({
  className,
  side = "left",
  ...props
}: React.ComponentProps<"div"> & { side?: "left" | "right" }) {
  const { leftOpen, leftWidth, setLeftWidth, setLeftResizing } = useSidebarLeft()
  const { rightOpen, rightWidth, setRightWidth, setRightResizing } = useSidebarRight()

  const isOpen = side === "left" ? leftOpen : rightOpen
  const width = side === "left" ? leftWidth : rightWidth
  const setWidth = side === "left" ? setLeftWidth : setRightWidth
  const setResizing = side === "left" ? setLeftResizing : setRightResizing

  const [isResizing, setIsResizing] = React.useState(false)
  const startXRef = React.useRef(0)
  const startWidthRef = React.useRef(0)

  const handleMouseDown = React.useCallback(
    (e: React.MouseEvent) => {
      if (!isOpen) return
      e.preventDefault()
      setIsResizing(true)
      setResizing(true)
      startXRef.current = e.clientX
      startWidthRef.current = width
      document.body.style.cursor = "ew-resize"
      document.body.style.userSelect = "none"
    },
    [isOpen, width, setResizing]
  )

  React.useEffect(() => {
    if (!isResizing) return

    const handleMouseMove = (e: MouseEvent) => {
      const delta =
        side === "left"
          ? e.clientX - startXRef.current
          : startXRef.current - e.clientX
      const maxWidth = getSidebarMaxWidth(side)
      const minWidth = getSidebarMinWidth(side)
      const newWidth = Math.max(
        minWidth,
        Math.min(maxWidth, startWidthRef.current + delta)
      )
      setWidth(newWidth)
    }

    const handleMouseUp = () => {
      setIsResizing(false)
      setResizing(false)
      document.body.style.cursor = ""
      document.body.style.userSelect = ""
    }

    window.addEventListener("mousemove", handleMouseMove)
    window.addEventListener("mouseup", handleMouseUp)
    return () => {
      window.removeEventListener("mousemove", handleMouseMove)
      window.removeEventListener("mouseup", handleMouseUp)
    }
  }, [isResizing, side, setWidth, setResizing])

  const handleDoubleClick = React.useCallback(() => {
    setWidth(
      side === "left" ? SIDEBAR_WIDTH_DEFAULT_LEFT : SIDEBAR_WIDTH_DEFAULT_RIGHT
    )
  }, [side, setWidth])

  return (
    <div
      data-sidebar="rail"
      data-slot="sidebar-rail"
      aria-label="Resize Sidebar"
      role="separator"
      onMouseDown={handleMouseDown}
      onDoubleClick={handleDoubleClick}
      title="Drag to resize, double-click to reset"
      className={cn(
        "absolute inset-y-0 z-20 w-1.5 cursor-ew-resize transition-colors ease-linear",
        side === "left"
          ? "right-0 translate-x-1/2"
          : "left-0 -translate-x-1/2",
        isResizing && "bg-primary",
        "hover:bg-sidebar-border",
        !isOpen && "hidden",
        className
      )}
      {...props}
    />
  )
}

function SidebarInset({ className, ...props }: React.ComponentProps<"main">) {
  return (
    <main
      data-slot="sidebar-inset"
      className={cn(
        "bg-background md:peer-data-[variant=inset]:m-2 md:peer-data-[variant=inset]:ml-0 md:peer-data-[variant=inset]:rounded-xl md:peer-data-[variant=inset]:shadow-sm md:peer-data-[variant=inset]:peer-data-[state=collapsed]:ml-2 relative flex w-full flex-1 flex-col",
        className
      )}
      {...props}
    />
  )
}

function SidebarInput({
  className,
  ...props
}: React.ComponentProps<typeof Input>) {
  return (
    <Input
      data-slot="sidebar-input"
      data-sidebar="input"
      className={cn("bg-muted/20 dark:bg-muted/30 border-input h-8 w-full", className)}
      {...props}
    />
  )
}

function SidebarHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-header"
      data-sidebar="header"
      className={cn("gap-2 p-2 flex flex-col", className)}
      {...props}
    />
  )
}

function SidebarFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-footer"
      data-sidebar="footer"
      className={cn("gap-2 p-2 flex flex-col", className)}
      {...props}
    />
  )
}

function SidebarSeparator({
  className,
  ...props
}: React.ComponentProps<typeof Separator>) {
  return (
    <Separator
      data-slot="sidebar-separator"
      data-sidebar="separator"
      className={cn("bg-sidebar-border mx-2 w-auto", className)}
      {...props}
    />
  )
}

function SidebarContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-content"
      data-sidebar="content"
      className={cn(
        "no-scrollbar gap-0 flex min-h-0 flex-1 flex-col overflow-auto group-data-[collapsible=icon]:overflow-hidden",
        className
      )}
      {...props}
    />
  )
}

function SidebarGroup({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-group"
      data-sidebar="group"
      className={cn(
        "px-2 py-1 relative flex w-full min-w-0 flex-col",
        className
      )}
      {...props}
    />
  )
}

function SidebarGroupLabel({
  className,
  asChild = false,
  ...props
}: React.ComponentProps<"div"> & { asChild?: boolean }) {
  const Comp = asChild ? Slot.Root : "div"

  return (
    <Comp
      data-slot="sidebar-group-label"
      data-sidebar="group-label"
      className={cn(
        "text-sidebar-foreground/70 ring-sidebar-ring h-8 rounded-md px-2 text-xs transition-[margin,opacity] duration-200 ease-linear group-data-[collapsible=icon]:-mt-8 group-data-[collapsible=icon]:opacity-0 focus-visible:ring-2 [&>svg]:size-4 flex shrink-0 items-center outline-hidden [&>svg]:shrink-0",
        className
      )}
      {...props}
    />
  )
}

function SidebarGroupAction({
  className,
  asChild = false,
  ...props
}: React.ComponentProps<"button"> & { asChild?: boolean }) {
  const Comp = asChild ? Slot.Root : "button"

  return (
    <Comp
      data-slot="sidebar-group-action"
      data-sidebar="group-action"
      className={cn(
        "text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground absolute top-3.5 right-3 w-5 rounded-md p-0 focus-visible:ring-2 [&>svg]:size-4 flex aspect-square items-center justify-center outline-hidden transition-transform group-data-[collapsible=icon]:hidden after:absolute after:-inset-2 md:after:hidden [&>svg]:shrink-0",
        className
      )}
      {...props}
    />
  )
}

function SidebarGroupContent({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-group-content"
      data-sidebar="group-content"
      className={cn("text-xs w-full", className)}
      {...props}
    />
  )
}

function SidebarMenu({ className, ...props }: React.ComponentProps<"ul">) {
  return (
    <ul
      data-slot="sidebar-menu"
      data-sidebar="menu"
      className={cn("gap-px flex w-full min-w-0 flex-col", className)}
      {...props}
    />
  )
}

function SidebarMenuItem({ className, ...props }: React.ComponentProps<"li">) {
  return (
    <li
      data-slot="sidebar-menu-item"
      data-sidebar="menu-item"
      className={cn("group/menu-item relative", className)}
      {...props}
    />
  )
}

const sidebarMenuButtonVariants = cva(
  "ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground active:bg-sidebar-accent active:text-sidebar-accent-foreground data-active:bg-sidebar-accent data-active:text-sidebar-accent-foreground data-open:hover:bg-sidebar-accent data-open:hover:text-sidebar-accent-foreground gap-2 rounded-[calc(var(--radius-sm)+2px)] p-2 text-left text-xs transition-[width,height,padding] group-has-data-[sidebar=menu-action]/menu-item:pr-8 group-data-[collapsible=icon]:size-8! group-data-[collapsible=icon]:p-2! focus-visible:ring-2 data-active:font-medium peer/menu-button flex w-full items-center overflow-hidden outline-hidden group/menu-button disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 [&>span:last-child]:truncate [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
        outline: "bg-background hover:bg-sidebar-accent hover:text-sidebar-accent-foreground shadow-[0_0_0_1px_hsl(var(--sidebar-border))] hover:shadow-[0_0_0_1px_hsl(var(--sidebar-accent))]",
      },
      size: {
        default: "h-8 text-xs",
        sm: "h-7 text-xs",
        lg: "h-12 text-xs group-data-[collapsible=icon]:p-0!",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

function SidebarMenuButton({
  asChild = false,
  isActive = false,
  variant = "default",
  size = "default",
  tooltip,
  className,
  ...props
}: React.ComponentProps<"button"> & {
  asChild?: boolean
  isActive?: boolean
  tooltip?: string | React.ComponentProps<typeof TooltipContent>
} & VariantProps<typeof sidebarMenuButtonVariants>) {
  const Comp = asChild ? Slot.Root : "button"
  const { isMobile } = useSidebarMobile()
  const { leftState: state } = useSidebarLeft()

  const button = (
    <Comp
      data-slot="sidebar-menu-button"
      data-sidebar="menu-button"
      data-size={size}
      data-active={isActive}
      className={cn(sidebarMenuButtonVariants({ variant, size }), className)}
      {...props}
    />
  )

  if (!tooltip) {
    return button
  }

  if (typeof tooltip === "string") {
    tooltip = {
      children: tooltip,
    }
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>{button}</TooltipTrigger>
      <TooltipContent
        side="right"
        align="center"
        hidden={state !== "collapsed" || isMobile}
        {...tooltip}
      />
    </Tooltip>
  )
}

function SidebarMenuAction({
  className,
  asChild = false,
  showOnHover = false,
  ...props
}: React.ComponentProps<"button"> & {
  asChild?: boolean
  showOnHover?: boolean
}) {
  const Comp = asChild ? Slot.Root : "button"

  return (
    <Comp
      data-slot="sidebar-menu-action"
      data-sidebar="menu-action"
      className={cn(
        "text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground peer-hover/menu-button:text-sidebar-accent-foreground absolute top-1.5 right-1 aspect-square w-5 rounded-[calc(var(--radius-sm)-2px)] p-0 peer-data-[size=default]/menu-button:top-1.5 peer-data-[size=lg]/menu-button:top-2.5 peer-data-[size=sm]/menu-button:top-1 focus-visible:ring-2 [&>svg]:size-4 flex items-center justify-center outline-hidden transition-transform group-data-[collapsible=icon]:hidden after:absolute after:-inset-2 md:after:hidden [&>svg]:shrink-0",
        showOnHover &&
          "peer-data-active/menu-button:text-sidebar-accent-foreground group-focus-within/menu-item:opacity-100 group-hover/menu-item:opacity-100 aria-expanded:opacity-100 md:opacity-0",
        className
      )}
      {...props}
    />
  )
}

function SidebarMenuBadge({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-menu-badge"
      data-sidebar="menu-badge"
      className={cn(
        "text-sidebar-foreground peer-hover/menu-button:text-sidebar-accent-foreground peer-data-active/menu-button:text-sidebar-accent-foreground pointer-events-none absolute right-1 h-5 min-w-5 rounded-[calc(var(--radius-sm)-2px)] px-1 text-xs font-medium peer-data-[size=default]/menu-button:top-1.5 peer-data-[size=lg]/menu-button:top-2.5 peer-data-[size=sm]/menu-button:top-1 flex items-center justify-center tabular-nums select-none group-data-[collapsible=icon]:hidden",
        className
      )}
      {...props}
    />
  )
}

function SidebarMenuSkeleton({
  className,
  showIcon = false,
  ...props
}: React.ComponentProps<"div"> & {
  showIcon?: boolean
}) {
  // Random width between 50 to 90%.
  const [width] = React.useState(() => {
    return `${Math.floor(Math.random() * 40) + 50}%`
  })

  return (
    <div
      data-slot="sidebar-menu-skeleton"
      data-sidebar="menu-skeleton"
      className={cn("h-8 gap-2 rounded-md px-2 flex items-center", className)}
      {...props}
    >
      {showIcon && (
        <Skeleton
          className="size-4 rounded-md"
          data-sidebar="menu-skeleton-icon"
        />
      )}
      <Skeleton
        className="h-4 max-w-(--skeleton-width) flex-1"
        data-sidebar="menu-skeleton-text"
        style={
          {
            "--skeleton-width": width,
          } as React.CSSProperties
        }
      />
    </div>
  )
}

function SidebarMenuSub({ className, ...props }: React.ComponentProps<"ul">) {
  return (
    <ul
      data-slot="sidebar-menu-sub"
      data-sidebar="menu-sub"
      className={cn("border-sidebar-border mx-3.5 translate-x-px gap-1 border-l px-2.5 py-0.5 group-data-[collapsible=icon]:hidden flex min-w-0 flex-col", className)}
      {...props}
    />
  )
}

function SidebarMenuSubItem({
  className,
  ...props
}: React.ComponentProps<"li">) {
  return (
    <li
      data-slot="sidebar-menu-sub-item"
      data-sidebar="menu-sub-item"
      className={cn("group/menu-sub-item relative", className)}
      {...props}
    />
  )
}

function SidebarMenuSubButton({
  asChild = false,
  size = "md",
  isActive = false,
  className,
  ...props
}: React.ComponentProps<"a"> & {
  asChild?: boolean
  size?: "sm" | "md"
  isActive?: boolean
}) {
  const Comp = asChild ? Slot.Root : "a"

  return (
    <Comp
      data-slot="sidebar-menu-sub-button"
      data-sidebar="menu-sub-button"
      data-size={size}
      data-active={isActive}
      className={cn(
        "text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground active:bg-sidebar-accent active:text-sidebar-accent-foreground [&>svg]:text-sidebar-accent-foreground data-active:bg-sidebar-accent data-active:text-sidebar-accent-foreground h-7 gap-2 rounded-md px-2 focus-visible:ring-2 data-[size=md]:text-xs data-[size=sm]:text-xs [&>svg]:size-4 flex min-w-0 -translate-x-px items-center overflow-hidden outline-hidden group-data-[collapsible=icon]:hidden disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 [&>span:last-child]:truncate [&>svg]:shrink-0",
        className
      )}
      {...props}
    />
  )
}

export {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInput,
  SidebarInset,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSkeleton,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  SidebarProvider,
  SidebarRail,
  SidebarSeparator,
  SidebarTrigger,
  useSidebarLeft,
  useSidebarMobile,
  useSidebarRight,
  useSidebar,
}
