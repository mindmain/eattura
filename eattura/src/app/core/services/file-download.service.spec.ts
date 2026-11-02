import { describe, it, expect, vi } from "vitest";
import { Injector, PLATFORM_ID } from "@angular/core";
import { FileDownloadService } from "./file-download.service";

describe("FileDownloadService", () => {
  it("crea un blob e simula il click in ambiente browser", () => {
    const injector = Injector.create({
      providers: [
        FileDownloadService,
        { provide: PLATFORM_ID, useValue: "browser" },
      ],
    });

    const service = injector.get(FileDownloadService);

    const createObjectURLSpy = vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:mock-url");
    const revokeObjectURLSpy = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});

    service.downloadTextFile("<xml>content</xml>", "fattura.xml");

    expect(createObjectURLSpy).toHaveBeenCalled();
    expect(revokeObjectURLSpy).toHaveBeenCalledWith("blob:mock-url");
    expect(clickSpy).toHaveBeenCalledOnce();

    createObjectURLSpy.mockRestore();
    revokeObjectURLSpy.mockRestore();
    clickSpy.mockRestore();
  });

  it("non solleva errori ed esce anticipatamente se non in browser (es. server)", () => {
    const injector = Injector.create({
      providers: [
        FileDownloadService,
        { provide: PLATFORM_ID, useValue: "server" },
      ],
    });

    const service = injector.get(FileDownloadService);
    const createObjectURLSpy = vi.spyOn(URL, "createObjectURL");

    expect(() => {
      service.downloadTextFile("<xml>test</xml>", "test.xml");
    }).not.toThrow();

    expect(createObjectURLSpy).not.toHaveBeenCalled();
    createObjectURLSpy.mockRestore();
  });
});
