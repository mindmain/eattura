import { DOCUMENT } from "@angular/common";
import { ChangeDetectionStrategy, Component, inject, OnDestroy, OnInit } from "@angular/core";
import { RouterLink, RouterOutlet } from "@angular/router";
import { SidebarComponent } from "./shared/components/sidebar/sidebar.component";
import { NotificationService } from "./core/services/notification.service";
import { InvoiceStore } from "./core/stores/invoice.store";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ThemeService } from "./core/services/theme.service";
import { LucideAngularModule, Settings2, UserRound } from "lucide-angular";
import { McpService } from "./core/services/mcp.service";

@Component({
  selector: "app-root",
  standalone: true,
  imports: [RouterLink, RouterOutlet, SidebarComponent, LucideAngularModule],
  templateUrl: "./app.component.html",
  styleUrl: "./app.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AppComponent implements OnInit, OnDestroy {
  protected notifications = inject(NotificationService);
  private invoices = inject(InvoiceStore);
  private readonly mcp = inject(McpService);
  private readonly theme = inject(ThemeService);
  private readonly document = inject(DOCUMENT);
  protected readonly macOSOverlay = isTauri() && /Macintosh|Mac OS X/.test(this.document.defaultView?.navigator.userAgent ?? "");
  protected readonly icons = { Settings2, UserRound };

  ngOnInit(): void {
    this.invoices.startPecMonitor();
    void this.mcp.start();
  }

  ngOnDestroy(): void {
    this.invoices.stopPecMonitor();
    this.mcp.stop();
  }

  async startDragging(event: MouseEvent): Promise<void> {
    if (this.macOSOverlay && event.button === 0) await getCurrentWindow().startDragging();
  }
}
