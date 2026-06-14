using Blazor.Diagrams;
using Blazor.Diagrams.Core.Models.Base;
using Blazor.Diagrams.Options;
using Microsoft.AspNetCore.Components;
using SemanticGraph.Visualizer.Client.Components.Graph;
using SemanticGraph.Visualizer.Client.Models;
using SemanticGraph.Visualizer.Client.Services;

namespace SemanticGraph.Visualizer.Client.Pages;

public partial class Home : IDisposable
{
    private const int ProjectionLimit = 150;

    [Inject]
    private GraphProjectionClient GraphClient { get; set; } = null!;

    private BlazorDiagram? Diagram { get; set; }

    private GraphProjectionDto? Projection { get; set; }

    private bool IsLoading { get; set; }

    private string? ErrorMessage { get; set; }

    private SemanticGraphNodeModel? SelectedNode { get; set; }

    private SemanticGraphLinkModel? SelectedEdge { get; set; }

    private bool IsEmpty => Projection is { Nodes.Count: 0 };

    private string DatabasePathText => Projection?.Metadata.DatabasePath ?? ".local/rust-workspace-extract-new.db";

    private string NodeCountText => (Projection?.Metadata.NodeCount ?? 0).ToString();

    private string EdgeCountText => (Projection?.Metadata.EdgeCount ?? 0).ToString();

    private string RelationSummaryText => Projection is null
        ? "none"
        : string.Join(", ", Projection.Edges
            .GroupBy(edge => edge.Relation)
            .OrderBy(group => group.Key)
            .Select(group => $"{group.Key}: {group.Count()}"));

    protected override async Task OnInitializedAsync()
    {
        Diagram = CreateDiagram();
        Diagram.SelectionChanged += OnSelectionChanged;
        await LoadProjectionAsync();
    }

    public void Dispose()
    {
        if (Diagram is not null)
        {
            Diagram.SelectionChanged -= OnSelectionChanged;
        }
    }

    private async Task ReloadAsync()
    {
        await LoadProjectionAsync();
    }

    private async Task LoadProjectionAsync()
    {
        IsLoading = true;
        ErrorMessage = null;

        try
        {
            var projection = await GraphClient.GetProjectionAsync(ProjectionLimit);
            Projection = projection;

            if (Diagram is not null)
            {
                GraphDiagramBuilder.Populate(Diagram, projection);
            }

            ClearSelection();
        }
        catch (Exception exception)
        {
            Projection = null;
            ErrorMessage = exception.Message;
            Diagram?.Links.Clear();
            Diagram?.Nodes.Clear();
            ClearSelection();
        }
        finally
        {
            IsLoading = false;
        }
    }

    private static BlazorDiagram CreateDiagram()
    {
        var options = new BlazorDiagramOptions
        {
            AllowMultiSelection = false,
            GridSize = 20,
            Zoom =
            {
                Minimum = 0.2
            },
            Virtualization =
            {
                Enabled = true
            }
        };

        var diagram = new BlazorDiagram(options);
        diagram.RegisterComponent<SemanticGraphNodeModel, GraphNodeWidget>();

        return diagram;
    }

    private void OnSelectionChanged(SelectableModel model)
    {
        if (Diagram is null)
        {
            return;
        }

        var selectedModel = model.Selected
            ? model
            : Diagram.GetSelectedModels().LastOrDefault();

        SelectedNode = selectedModel as SemanticGraphNodeModel;
        SelectedEdge = selectedModel as SemanticGraphLinkModel;

        _ = InvokeAsync(StateHasChanged);
    }

    private void ClearSelection()
    {
        SelectedNode = null;
        SelectedEdge = null;
    }
}
