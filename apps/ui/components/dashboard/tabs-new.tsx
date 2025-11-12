import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ReactNode } from "react"

export interface TabItem {
  value: string
  label: string
  icon?: ReactNode
  disabled?: boolean
}

export interface TabContentItem {
  value: string
  content: ReactNode
}

interface ReusableTabsProps {
  tabs: TabItem[]
  contents: TabContentItem[]
  defaultValue?: string
  className?: string
  tabsListClassName?: string
  tabsTriggerClassName?: string
  tabsContentClassName?: string
}

export function ReusableTabs({
  tabs,
  contents,
  defaultValue,
  className = "space-y-4",
  tabsListClassName = "bg-secondary gap-1",
  tabsTriggerClassName = "px-4",
  tabsContentClassName = "space-y-4",
}: ReusableTabsProps) {
  const defaultTab = defaultValue || tabs[0]?.value

  return (
    <Tabs defaultValue={defaultTab} className={className}>
      <TabsList className={tabsListClassName}>
        {tabs.map((tab) => (
          <TabsTrigger
            key={tab.value}
            value={tab.value}
            className={tabsTriggerClassName}
            disabled={tab.disabled}
          >
            {tab.icon && <span className="mr-2">{tab.icon}</span>}
            {tab.label}
          </TabsTrigger>
        ))}
      </TabsList>

      {contents.map((content) => (
        <TabsContent
          key={content.value}
          value={content.value}
          className={tabsContentClassName}
        >
          {content.content}
        </TabsContent>
      ))}
    </Tabs>
  )
}