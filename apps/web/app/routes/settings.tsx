import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@revlentless/ui/components/tabs";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { Input } from "@revlentless/ui/components/input";
import { Label } from "@revlentless/ui/components/label";
import { Switch } from "@revlentless/ui/components/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@revlentless/ui/components/select";
import { Button } from "@revlentless/ui/components/button";
import { Slider } from "@revlentless/ui/components/slider";
import { Separator } from "@revlentless/ui/components/separator";
import { useState } from "react";
import { toast } from "@revlentless/ui/components/sonner";

export default function Settings() {
  const [micGain, setMicGain] = useState([70]);

  function handleSave() {
    toast("Settings saved", {
      description: "Your preferences have been updated.",
    });
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Settings</h1>
        <p className="text-muted-foreground">
          Manage your account and app preferences.
        </p>
      </div>

      <Tabs defaultValue="profile" className="space-y-6">
        <TabsList>
          <TabsTrigger value="profile">Profile</TabsTrigger>
          <TabsTrigger value="preferences">Preferences</TabsTrigger>
          <TabsTrigger value="audio">Audio</TabsTrigger>
          <TabsTrigger value="account">Account</TabsTrigger>
        </TabsList>

        <TabsContent value="profile">
          <Card>
            <CardHeader>
              <CardTitle>Profile</CardTitle>
              <CardDescription>Basic information about you.</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  placeholder="Ada Lovelace"
                  defaultValue="Ada Lovelace"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="email">Email</Label>
                <Input
                  id="email"
                  placeholder="ada@example.com"
                  defaultValue="ada@example.com"
                />
              </div>
              <div className="space-y-2 sm:col-span-2">
                <Label htmlFor="org">Organization</Label>
                <Input
                  id="org"
                  placeholder="Feynman Labs"
                  defaultValue="Feynman Labs"
                />
              </div>
              <div className="sm:col-span-2">
                <Button
                  onClick={handleSave}
                  className="bg-emerald-600 hover:bg-emerald-700"
                >
                  Save changes
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="preferences">
          <Card>
            <CardHeader>
              <CardTitle>App Preferences</CardTitle>
              <CardDescription>
                Customize appearance and interactions.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-6">
              <div className="flex items-center justify-between">
                <div>
                  <div className="font-medium">Compact mode</div>
                  <div className="text-sm text-muted-foreground">
                    Reduce spacing for dense displays.
                  </div>
                </div>
                <Switch />
              </div>
              <Separator />
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label>Default landing</Label>
                  <Select defaultValue="dashboard">
                    <SelectTrigger>
                      <SelectValue placeholder="Choose" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="dashboard">Dashboard</SelectItem>
                      <SelectItem value="sessions">Sessions</SelectItem>
                      <SelectItem value="topics">Topics</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label>Accent color</Label>
                  <Select defaultValue="emerald">
                    <SelectTrigger>
                      <SelectValue placeholder="Choose" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="emerald">Emerald</SelectItem>
                      <SelectItem value="amber">Amber</SelectItem>
                      <SelectItem value="gray">Gray</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div>
                <Button
                  onClick={handleSave}
                  className="bg-emerald-600 hover:bg-emerald-700"
                >
                  Save preferences
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="audio">
          <Card>
            <CardHeader>
              <CardTitle>Audio</CardTitle>
              <CardDescription>
                Configure microphone input and playback.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label>Microphone</Label>
                  <Select defaultValue="default">
                    <SelectTrigger>
                      <SelectValue placeholder="Select input" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="default">
                        Default Microphone
                      </SelectItem>
                      <SelectItem value="studio">Studio Mic</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label>Playback Device</Label>
                  <Select defaultValue="speakers">
                    <SelectTrigger>
                      <SelectValue placeholder="Select output" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="speakers">Speakers</SelectItem>
                      <SelectItem value="headphones">Headphones</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="space-y-2">
                <Label>Mic Gain</Label>
                <Slider value={micGain} onValueChange={setMicGain} />
              </div>
              <Button
                onClick={handleSave}
                className="bg-emerald-600 hover:bg-emerald-700"
              >
                Save audio settings
              </Button>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="account">
          <Card>
            <CardHeader>
              <CardTitle>Account</CardTitle>
              <CardDescription>
                Manage account-level preferences.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label>Password</Label>
                <Input
                  type="password"
                  placeholder="••••••••"
                  defaultValue="password"
                />
              </div>
              <div className="space-y-2">
                <Label>Time Zone</Label>
                <Select defaultValue="utc">
                  <SelectTrigger>
                    <SelectValue placeholder="Time zone" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="utc">UTC</SelectItem>
                    <SelectItem value="pst">PST</SelectItem>
                    <SelectItem value="cet">CET</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="sm:col-span-2">
                <Button onClick={handleSave} variant="outline">
                  Save account settings
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
