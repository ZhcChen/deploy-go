//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/application_template_file_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_template_response.g.dart';

/// ApplicationTemplateResponse
///
/// Properties:
/// * [defaultImage]
/// * [defaultPort]
/// * [deploymentMechanism]
/// * [digest]
/// * [files]
/// * [id]
/// * [name]
/// * [summary]
/// * [version]
@BuiltValue()
abstract class ApplicationTemplateResponse implements Built<ApplicationTemplateResponse, ApplicationTemplateResponseBuilder> {
  @BuiltValueField(wireName: r'default_image')
  String get defaultImage;

  @BuiltValueField(wireName: r'default_port')
  int get defaultPort;

  @BuiltValueField(wireName: r'deployment_mechanism')
  String get deploymentMechanism;

  @BuiltValueField(wireName: r'digest')
  String get digest;

  @BuiltValueField(wireName: r'files')
  BuiltList<ApplicationTemplateFileResponse> get files;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'summary')
  String get summary;

  @BuiltValueField(wireName: r'version')
  String get version;

  ApplicationTemplateResponse._();

  factory ApplicationTemplateResponse([void updates(ApplicationTemplateResponseBuilder b)]) = _$ApplicationTemplateResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationTemplateResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationTemplateResponse> get serializer => _$ApplicationTemplateResponseSerializer();
}

class _$ApplicationTemplateResponseSerializer implements PrimitiveSerializer<ApplicationTemplateResponse> {
  @override
  final Iterable<Type> types = const [ApplicationTemplateResponse, _$ApplicationTemplateResponse];

  @override
  final String wireName = r'ApplicationTemplateResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationTemplateResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'default_image';
    yield serializers.serialize(
      object.defaultImage,
      specifiedType: const FullType(String),
    );
    yield r'default_port';
    yield serializers.serialize(
      object.defaultPort,
      specifiedType: const FullType(int),
    );
    yield r'deployment_mechanism';
    yield serializers.serialize(
      object.deploymentMechanism,
      specifiedType: const FullType(String),
    );
    yield r'digest';
    yield serializers.serialize(
      object.digest,
      specifiedType: const FullType(String),
    );
    yield r'files';
    yield serializers.serialize(
      object.files,
      specifiedType: const FullType(BuiltList, [FullType(ApplicationTemplateFileResponse)]),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'summary';
    yield serializers.serialize(
      object.summary,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationTemplateResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationTemplateResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'default_image':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.defaultImage = valueDes;
          break;
        case r'default_port':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.defaultPort = valueDes;
          break;
        case r'deployment_mechanism':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.deploymentMechanism = valueDes;
          break;
        case r'digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.digest = valueDes;
          break;
        case r'files':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ApplicationTemplateFileResponse)]),
          ) as BuiltList<ApplicationTemplateFileResponse>;
          result.files.replace(valueDes);
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'summary':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.summary = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationTemplateResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationTemplateResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
