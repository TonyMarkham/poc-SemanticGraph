using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Radzen;
using SemanticGraph.Visualizer.Client;
using SemanticGraph.Visualizer.Client.Services;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

var backendBaseUrl = builder.Configuration["SemanticGraph:BackendBaseUrl"] ?? builder.HostEnvironment.BaseAddress;
builder.Services.AddRadzenComponents();
builder.Services.AddScoped(_ => new HttpClient { BaseAddress = CreateBaseUri(backendBaseUrl) });
builder.Services.AddScoped<GraphProjectionClient>();

await builder.Build().RunAsync();

static Uri CreateBaseUri(string value)
{
    return new Uri(value.EndsWith("/", StringComparison.Ordinal) ? value : $"{value}/");
}
