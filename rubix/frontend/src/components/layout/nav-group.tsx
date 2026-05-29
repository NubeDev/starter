import { type ReactNode } from 'react'
import { Link, useLocation } from '@tanstack/react-router'
import { ChevronRight } from 'lucide-react'
import { useIntl } from 'react-intl'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible'
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from '@/components/ui/sidebar'
import { Badge } from '../ui/badge'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu'
import {
  type NavCollapsible,
  type NavItem,
  type NavLink,
  type NavGroup as NavGroupProps,
} from './types'

export function NavGroup({ title, items }: NavGroupProps) {
  const { state, isMobile } = useSidebar()
  const href = useLocation({ select: (location) => location.href })
  const intl = useIntl()
  return (
    <SidebarGroup>
      <SidebarGroupLabel>{intl.formatMessage({ id: title })}</SidebarGroupLabel>
      <SidebarMenu>
        {items.map((item) => {
          const key = `${item.title}-${item.url}`

          if (!item.items) return <SidebarMenuLink key={key} item={item} href={href} />

          if (state === 'collapsed' && !isMobile)
            return <SidebarMenuCollapsedDropdown key={key} item={item} href={href} />

          return <SidebarMenuCollapsible key={key} item={item} href={href} />
        })}
      </SidebarMenu>
    </SidebarGroup>
  )
}

function NavBadge({ children }: { children: ReactNode }) {
  return <Badge className='rounded-full px-1 py-0 text-xs'>{children}</Badge>
}

/** Route-aware link: TanStack `<Link>` for `/...` URLs, plain `<a>` for hash links. */
function NavAnchor({
  url,
  onClick,
  children,
  className,
}: {
  url: string
  onClick?: () => void
  children: ReactNode
  className?: string
}) {
  if (url.startsWith('/')) {
    return (
      <Link to={url} onClick={onClick} className={className}>
        {children}
      </Link>
    )
  }
  return (
    <a
      href={url}
      onClick={(e) => {
        e.preventDefault()
        onClick?.()
      }}
      className={className}
    >
      {children}
    </a>
  )
}

function SidebarMenuLink({ item, href }: { item: NavLink; href: string }) {
  const { setOpenMobile } = useSidebar()
  const intl = useIntl()
  const label = intl.formatMessage({ id: item.title, defaultMessage: item.defaultMessage })
  return (
    <SidebarMenuItem>
      <SidebarMenuButton asChild isActive={checkIsActive(href, item)} tooltip={label}>
        <NavAnchor url={item.url as string} onClick={() => setOpenMobile(false)}>
          {item.icon && <item.icon />}
          <span>{label}</span>
          {item.badge && <NavBadge>{item.badge}</NavBadge>}
        </NavAnchor>
      </SidebarMenuButton>
    </SidebarMenuItem>
  )
}

function SidebarMenuCollapsible({ item, href }: { item: NavCollapsible; href: string }) {
  const { setOpenMobile } = useSidebar()
  const intl = useIntl()
  const label = intl.formatMessage({ id: item.title, defaultMessage: item.defaultMessage })
  return (
    <Collapsible
      asChild
      defaultOpen={checkIsActive(href, item, true)}
      className='group/collapsible'
    >
      <SidebarMenuItem>
        {item.url ? (
          <div className='flex items-center'>
            <SidebarMenuButton
              asChild
              isActive={checkIsActive(href, item as unknown as NavLink)}
              tooltip={label}
              className='flex-1'
            >
              <NavAnchor url={item.url} onClick={() => setOpenMobile(false)}>
                {item.icon && <item.icon />}
                <span>{label}</span>
                {item.badge && <NavBadge>{item.badge}</NavBadge>}
              </NavAnchor>
            </SidebarMenuButton>
            <CollapsibleTrigger asChild>
              <button
                type='button'
                aria-label={`Toggle ${label}`}
                className='grid size-7 place-items-center rounded-md text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground'
              >
                <ChevronRight className='size-4 transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90' />
              </button>
            </CollapsibleTrigger>
          </div>
        ) : (
          <CollapsibleTrigger asChild>
            <SidebarMenuButton tooltip={label}>
              {item.icon && <item.icon />}
              <span>{label}</span>
              {item.badge && <NavBadge>{item.badge}</NavBadge>}
              <ChevronRight className='ms-auto transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90' />
            </SidebarMenuButton>
          </CollapsibleTrigger>
        )}
        <CollapsibleContent className='CollapsibleContent'>
          <SidebarMenuSub>
            {item.items.map((subItem) => (
              <SidebarMenuSubItem key={subItem.title}>
                <SidebarMenuSubButton asChild isActive={checkIsActive(href, subItem)}>
                  <NavAnchor url={subItem.url as string} onClick={() => setOpenMobile(false)}>
                    {subItem.icon && <subItem.icon />}
                    <span>{intl.formatMessage({ id: subItem.title, defaultMessage: subItem.defaultMessage })}</span>
                    {subItem.badge && <NavBadge>{subItem.badge}</NavBadge>}
                  </NavAnchor>
                </SidebarMenuSubButton>
              </SidebarMenuSubItem>
            ))}
          </SidebarMenuSub>
        </CollapsibleContent>
      </SidebarMenuItem>
    </Collapsible>
  )
}

function SidebarMenuCollapsedDropdown({ item, href }: { item: NavCollapsible; href: string }) {
  const intl = useIntl()
  const label = intl.formatMessage({ id: item.title, defaultMessage: item.defaultMessage })
  return (
    <SidebarMenuItem>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <SidebarMenuButton tooltip={label} isActive={checkIsActive(href, item)}>
            {item.icon && <item.icon />}
            <span>{label}</span>
            {item.badge && <NavBadge>{item.badge}</NavBadge>}
            <ChevronRight className='ms-auto transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90' />
          </SidebarMenuButton>
        </DropdownMenuTrigger>
        <DropdownMenuContent side='right' align='start' sideOffset={4}>
          <DropdownMenuLabel>
            {label} {item.badge ? `(${item.badge})` : ''}
          </DropdownMenuLabel>
          <DropdownMenuSeparator />
          {item.items.map((sub) => (
            <DropdownMenuItem key={`${sub.title}-${sub.url}`} asChild>
              <NavAnchor
                url={sub.url as string}
                className={checkIsActive(href, sub) ? 'bg-[color:var(--color-surface-2)]/60' : ''}
              >
                {sub.icon && <sub.icon />}
                <span className='max-w-52 text-wrap'>{intl.formatMessage({ id: sub.title })}</span>
                {sub.badge && <span className='ms-auto text-xs'>{sub.badge}</span>}
              </NavAnchor>
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </SidebarMenuItem>
  )
}

function checkIsActive(href: string, item: NavItem, mainNav = false) {
  return (
    href === item.url ||
    href.split('?')[0] === item.url ||
    !!item?.items?.filter((i) => i.url === href).length ||
    (mainNav && href.split('/')[1] !== '' && href.split('/')[1] === item?.url?.split('/')[1])
  )
}
