import { useNavigate } from 'react-router'
import { useState } from 'react'
import { Button } from '@revlentless/ui/components/button'
import { Input } from '@revlentless/ui/components/input'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@revlentless/ui/components/card'
import { Badge } from '@revlentless/ui/components/badge'
import { Rocket, BookOpenText, Mic, ChevronRight } from 'lucide-react'

export default function Home() {
  const navigate = useNavigate()
  const [topic, setTopic] = useState('Operating Systems')

  function handleStart(e: React.FormEvent) {
    e.preventDefault()
    const t = topic?.trim() || 'Operating Systems'
    navigate(`/sessions/new?topic=${encodeURIComponent(t)}`)
  }

  return (
    <main className="min-h-[100svh] flex items-center justify-center px-4 py-10 bg-gradient-to-b from-white to-emerald-50/30">
      <div className="w-full max-w-3xl">
        <div className="mb-10 text-center">
          <div className="inline-flex items-center gap-2 rounded-full border bg-white px-3 py-1 text-sm text-muted-foreground">
            <Rocket className="h-4 w-4 text-emerald-600" aria-hidden="true" />
            <span>New</span>
            <span className="hidden sm:inline">Voice-first learning assistant</span>
          </div>
          <h1 className="mt-4 text-4xl font-bold tracking-tight sm:text-5xl">
            Feynman AI Teacher
          </h1>
          <p className="mx-auto mt-3 max-w-2xl text-muted-foreground">
            {'"The best way to learn is to teach."'} Explain a topic out loud; your curious AI student will ask questions and help you discover gaps.
          </p>
        </div>

        <Card className="shadow-md">
          <CardHeader className="space-y-2">
            <div className="mx-auto inline-flex h-12 w-12 items-center justify-center rounded-full bg-emerald-50 text-emerald-600">
              <BookOpenText aria-hidden="true" />
            </div>
            <CardTitle className="text-center text-2xl">Start teaching</CardTitle>
            <CardDescription className="text-center">
              Enter a topic and begin a session. We’ll mock the AI interactions for now.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleStart} className="space-y-4">
              <div className="space-y-2">
                <label htmlFor="topic" className="text-sm font-medium">
                  What topic do you want to teach today?
                </label>
                <Input
                  id="topic"
                  placeholder="e.g., Operating Systems, Linear Algebra, Carbon Markets"
                  value={topic}
                  onChange={(e) => setTopic(e.target.value)}
                />
              </div>
              <div className="flex flex-col gap-3">
                <Button type="submit" className="w-full bg-emerald-600 hover:bg-emerald-700">
                  <Mic className="mr-2 h-4 w-4" />
                  Start Teaching Session
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  className="w-full"
                  onClick={() => navigate('/dashboard')}
                >
                  Explore Dashboard
                  <ChevronRight className="ml-2 h-4 w-4" />
                </Button>
              </div>
              <div className="flex items-center justify-center gap-2 text-xs text-muted-foreground">
                <Badge variant="secondary">Mocked</Badge>
                <span>No server required. Data persists in your browser.</span>
              </div>
            </form>
          </CardContent>
        </Card>
      </div>
    </main>
  )
}
