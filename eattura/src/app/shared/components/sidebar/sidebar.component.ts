import { ChangeDetectionStrategy, Component } from "@angular/core";
import { RouterLink, RouterLinkActive } from "@angular/router";
import { LucideAngularModule, FileText, LayoutDashboard, UsersRound } from "lucide-angular";

@Component({
  selector: "app-sidebar",
  standalone: true,
  imports: [RouterLink, RouterLinkActive, LucideAngularModule],
  templateUrl: "./sidebar.component.html",
  styleUrl: "./sidebar.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SidebarComponent {
  protected readonly icons = { LayoutDashboard, UsersRound, FileText };
}
