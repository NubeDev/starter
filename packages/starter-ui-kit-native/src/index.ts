// Public surface of `@nube/starter-ui-kit-native`. Subpath imports
// (e.g. `@nube/starter-ui-kit-native/button`) work too — see the
// `exports` block in `package.json`. The barrel is here so renderer
// packages can `import { Button, Card, ... } from "..."` without
// hand-rolling every line.

export { useTheme } from "./theme.js";
export type { Theme, ThemePreferencesSnapshot } from "./theme.js";

export { Box, Row, Column, Text, ScrollArea, Divider } from "./layout.js";
export type {
  BoxProps,
  TextProps,
  ScrollAreaProps,
  DividerProps,
} from "./layout.js";

export { Button } from "./button.js";
export type { ButtonProps, ButtonVariant, ButtonSize } from "./button.js";

export {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardAction,
  CardContent,
  CardFooter,
} from "./card.js";
export type { CardProps } from "./card.js";

export { Input } from "./input.js";
export type { InputProps } from "./input.js";

export { Tabs, TabsList, TabsTrigger, TabsContent } from "./tabs.js";
export type { TabsProps, TabsTriggerProps, TabsContentProps } from "./tabs.js";

export { Badge } from "./badge.js";
export type { BadgeProps, BadgeVariant } from "./badge.js";

export { Switch } from "./switch.js";
export type { SwitchProps } from "./switch.js";

export { Slider } from "./slider.js";
export type { SliderProps } from "./slider.js";

export {
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
} from "./select.js";
export type {
  SelectProps,
  SelectTriggerProps,
  SelectItemProps,
} from "./select.js";

export {
  Sheet,
  SheetTrigger,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
  SheetClose,
} from "./sheet.js";
export type { SheetProps, SheetTriggerProps } from "./sheet.js";

export {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from "./dialog.js";
export type { DialogProps } from "./dialog.js";

export { Spinner } from "./spinner.js";
export type { SpinnerProps } from "./spinner.js";

export { Skeleton } from "./skeleton.js";
export type { SkeletonProps } from "./skeleton.js";

export {
  Tooltip,
  TooltipProvider,
  TooltipTrigger,
  TooltipContent,
} from "./tooltip.js";
export type {
  TooltipProps,
  TooltipProviderProps,
  TooltipTriggerProps,
  TooltipContentProps,
} from "./tooltip.js";
