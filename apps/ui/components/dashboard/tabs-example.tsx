// Contoh penggunaan ReusableTabs component

import { ReusableTabs, TabItem, TabContentItem } from "./tabs-new"
import { Code, FileText, BarChart, Calendar, Terminal } from "lucide-react"

export function TabsExample() {
  // 1. Define tabs dengan label, value, dan icon (optional)
  const tabs: TabItem[] = [
    { value: "editor", label: "Editor", icon: <Code size={16} /> },
    { value: "overview", label: "Overview", icon: <FileText size={16} /> },
    { value: "stats", label: "Stats", icon: <BarChart size={16} /> },
    { value: "events", label: "Events", icon: <Calendar size={16} /> },
    { value: "logs", label: "Logs", icon: <Terminal size={16} /> },
  ]

  // 2. Define content untuk setiap tab
  const contents: TabContentItem[] = [
    {
      value: "editor",
      content: (
        <div>
          <h3>Editor Content</h3>
          {/* Your editor component here */}
        </div>
      ),
    },
    {
      value: "overview",
      content: (
        <div>
          <h3>Overview Content</h3>
          {/* Your overview component here */}
        </div>
      ),
    },
    {
      value: "stats",
      content: (
        <div>
          <h3>Stats Content</h3>
          {/* Your stats component here */}
        </div>
      ),
    },
    {
      value: "events",
      content: (
        <div>
          <h3>Events Content</h3>
          {/* Your events component here */}
        </div>
      ),
    },
    {
      value: "logs",
      content: (
        <div>
          <h3>Logs Content</h3>
          {/* Your logs component here */}
        </div>
      ),
    },
  ]

  return (
    <ReusableTabs
      tabs={tabs}
      contents={contents}
      defaultValue="editor"
      className="space-y-4"
      tabsListClassName="bg-secondary gap-1"
      tabsTriggerClassName="px-4"
    />
  )
}

// ============================================
// Contoh 2: Tabs sederhana tanpa icon
// ============================================

export function SimpleTabsExample() {
  const tabs: TabItem[] = [
    { value: "tab1", label: "Tab 1" },
    { value: "tab2", label: "Tab 2" },
    { value: "tab3", label: "Tab 3" },
  ]

  const contents: TabContentItem[] = [
    { value: "tab1", content: <div>Content for Tab 1</div> },
    { value: "tab2", content: <div>Content for Tab 2</div> },
    { value: "tab3", content: <div>Content for Tab 3</div> },
  ]

  return <ReusableTabs tabs={tabs} contents={contents} />
}

// ============================================
// Contoh 3: Tabs dengan dynamic data dari props
// ============================================

interface DynamicTabsExampleProps {
  functionData: any
  functionId: string
  onComplete: () => void
}

export function DynamicTabsExample({
  functionData,
  functionId,
  onComplete,
}: DynamicTabsExampleProps) {
  const tabs: TabItem[] = [
    { value: "editor", label: "Editor" },
    { value: "overview", label: "Overview" },
    { value: "stats", label: "Stats" },
    { value: "events", label: "Events" },
    { value: "logs", label: "Logs" },
  ]

  const contents: TabContentItem[] = [
    {
      value: "editor",
      content: (
        <div>Editor dengan functionId: {functionId}</div>
        // <FunctionEditor
        //   functionData={functionData}
        //   mode="update"
        //   functionId={functionId}
        //   onComplete={onComplete}
        // />
      ),
    },
    {
      value: "overview",
      content: <div>Overview dengan data: {JSON.stringify(functionData)}</div>,
    },
    {
      value: "stats",
      content: <div>Stats content</div>,
    },
    {
      value: "events",
      content: <div>Events content</div>,
    },
    {
      value: "logs",
      content: <div>Logs untuk functionId: {functionId}</div>,
    },
  ]

  return <ReusableTabs tabs={tabs} contents={contents} defaultValue="editor" />
}
