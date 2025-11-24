import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ReactNode } from "react"

export interface TabItem {
  value: string
  label: string
  icon?: ReactNode
  badge?: ReactNode
  disabled?: boolean
}

export interface TabContentItem {
  value: string
  content: ReactNode
}

interface ReusableTabsProps {
  tabs: TabItem[]
  contents?: TabContentItem[]
  defaultValue?: string
  value?: string
  onValueChange?: (value: string) => void
  className?: string
  tabsListClassName?: string
  tabsTriggerClassName?: string
  tabsContentClassName?: string
}

export function ReusableTabs({
  tabs,
  contents,
  defaultValue,
  value,
  onValueChange,
  className = "space-y-4",
  tabsListClassName = "bg-secondary gap-1 h-auto dark:bg-[#4f1a00]",
  tabsTriggerClassName = "px-4 gap-1 data-[state=active]:dark:bg-card data-[state=active]:dark:text-primary dark:border-none",
  tabsContentClassName = "space-y-4",
}: ReusableTabsProps) {
  const defaultTab = defaultValue || tabs[0]?.value

  return (
    <Tabs
      defaultValue={defaultTab}
      value={value}
      onValueChange={onValueChange}
      className={className}
    >
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
            {tab.badge && <span className="ml-2">{tab.badge}</span>}
          </TabsTrigger>
        ))}
      </TabsList>

      {contents && contents.map((content) => (
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