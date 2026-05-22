import { Sidebar, SidebarInset, SidebarProvider, useSidebar } from '@/components/sidebar'
import Dashboard from '@/views/Dashboard'
import Showcase from '@/views/Showcase'

function RouteOutlet() {
  const { active } = useSidebar()
  switch (active) {
    case 'showcase':
      return <Showcase />
    case 'dashboard':
    default:
      return <Dashboard />
  }
}

export default function App() {
  return (
    <SidebarProvider>
      <Sidebar />
      <SidebarInset>
        <RouteOutlet />
      </SidebarInset>
    </SidebarProvider>
  )
}
